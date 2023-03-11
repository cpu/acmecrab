use crate::config::Shared;
use crate::error::Error;
use crate::txt_store::DynTxtStore;
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::net::IpAddr;
use time::macros::format_description;
use time::OffsetDateTime;
use tracing::error;
use trust_dns_proto::rr::rdata::SOA;
use trust_dns_server::authority::MessageResponseBuilder;
use trust_dns_server::client::op::{Header, MessageType, OpCode, ResponseCode};
use trust_dns_server::client::rr::rdata::TXT;
use trust_dns_server::client::rr::{LowerName, Name, RData, Record, RecordType};
use trust_dns_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};

#[derive(Clone)]
pub struct Handler {
    config: Shared,
    txt_domain_set: HashSet<LowerName>,
    txt_store: DynTxtStore,
}

lazy_static! {
    static ref SERIAL_FORMATTER: &'static [time::format_description::FormatItem<'static>] =
        format_description!(version = 2, "[year][month][day]");
}

impl Handler {
    pub(super) fn new(config: Shared, txt_store: DynTxtStore) -> Self {
        let txt_domain_set = Self::txt_domain_set(&config);
        Handler {
            config,
            txt_domain_set,
            txt_store,
        }
    }

    fn txt_domain_set(config: &Shared) -> HashSet<LowerName> {
        // Build a HashSet of all of the names that appear in the allowed network configs. We use
        // this to quickly determine whether to return NXDOMAIN for a TXT lookup.
        let mut txt_domain_set: HashSet<LowerName> = HashSet::default();
        for network_allowed_set in config.acl.values() {
            for subdomain in network_allowed_set {
                let subdomain: Name = subdomain.into();
                let fqdn = subdomain.append_domain(&(&config.domain).into()).unwrap();
                txt_domain_set.insert(fqdn.clone().into());
            }
        }
        txt_domain_set
    }

    async fn dispatch_request<R: ResponseHandler>(
        &self,
        request: &Request,
        response: R,
    ) -> Result<ResponseInfo, Error> {
        // If it isn't a query, return NOTIMPL.
        if request.op_code() != OpCode::Query || request.message_type() != MessageType::Query {
            return self.handle_notimpl(request, response).await;
        }

        // Otherwise handle by query type, or return NOTIMPL.
        match request.query().query_type() {
            RecordType::TXT => self.handle_request_txt(request, response).await,
            RecordType::SOA => self.handle_request_soa(request, response).await,
            RecordType::A => self.handle_request_a(request, response).await,
            RecordType::AAAA => self.handle_request_aaaa(request, response).await,
            RecordType::NS => self.handle_request_ns(request, response).await,
            _ => self.handle_notimpl(request, response).await,
        }
    }

    async fn handle_notimpl<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> Result<ResponseInfo, Error> {
        let response = MessageResponseBuilder::from_message_request(request);
        Ok(response_handle
            .send_response(response.error_msg(request.header(), ResponseCode::NotImp))
            .await?)
    }

    async fn handle_request_txt<R: ResponseHandler>(
        &self,
        request: &Request,
        response_handle: R,
    ) -> Result<ResponseInfo, Error> {
        let query_name = request.query().name();
        if self.txt_domain_set.get(query_name).is_none() {
            return self.send_nxdomain(request, response_handle).await;
        }

        let txt_data = self.txt_rdata(query_name).await;
        self.send_auth_resp(request, response_handle, txt_data)
            .await
    }

    async fn handle_request_soa<R: ResponseHandler>(
        &self,
        request: &Request,
        response_handle: R,
    ) -> Result<ResponseInfo, Error> {
        let query_name = request.query().name();
        if *query_name != self.config.domain {
            return self.send_nxdomain(request, response_handle).await;
        }

        // NB: unwraps are safe: known date format producing values that will always parse as u32.
        let serial: u32 = OffsetDateTime::now_utc()
            .format(&SERIAL_FORMATTER)
            .unwrap()
            .parse()
            .unwrap();
        let ns_admin = self.config.ns_admin()?;
        // See RIPE 203[0] for recommended values.
        // [0]: https://www.ripe.net/publications/docs/ripe-203
        let soa_rdata = RData::SOA(SOA::new(
            self.config.ns_domain.clone().into(),
            ns_admin,
            serial,
            86_400,    // 24 hrs.
            7_200,     // 2 hours.
            3_600_000, // 1000 hours.
            172_800,   // 2 days.
        ));
        self.send_auth_resp(request, response_handle, vec![soa_rdata])
            .await
    }

    async fn handle_request_a<R: ResponseHandler>(
        &self,
        request: &Request,
        response_handle: R,
    ) -> Result<ResponseInfo, Error> {
        let fqdn = request.query().name();
        match self.config.addrs.get(fqdn) {
            None => self.send_nxdomain(request, response_handle).await,
            Some(_) => {
                self.send_auth_resp(request, response_handle, self.a_rdata(fqdn))
                    .await
            }
        }
    }

    async fn handle_request_aaaa<R: ResponseHandler>(
        &self,
        request: &Request,
        response_handle: R,
    ) -> Result<ResponseInfo, Error> {
        let fqdn = request.query().name();
        match self.config.addrs.get(fqdn) {
            None => self.send_nxdomain(request, response_handle).await,
            Some(_) => {
                self.send_auth_resp(request, response_handle, self.aaaa_rdata(fqdn))
                    .await
            }
        }
    }

    async fn handle_request_ns<R: ResponseHandler>(
        &self,
        request: &Request,
        response_handle: R,
    ) -> Result<ResponseInfo, Error> {
        let fqdn = request.query().name();
        match self.config.ns_records.get(fqdn) {
            None => self.send_nxdomain(request, response_handle).await,
            Some(_) => {
                self.send_auth_resp(request, response_handle, self.ns_rdata(fqdn))
                    .await
            }
        }
    }

    async fn txt_rdata(&self, key: &LowerName) -> Vec<RData> {
        let read_store = self.txt_store.read().await;
        let res = read_store.get_txt(key).await;
        res.iter()
            .map(|s| RData::TXT(TXT::new(vec![s.to_string()])))
            .collect()
    }

    fn addrs_from_config(&self, fqdn: &LowerName) -> Vec<IpAddr> {
        self.config
            .addrs
            .get(fqdn)
            .map_or(Vec::default(), Clone::clone)
    }

    fn ns_names_from_config(&self, fqdn: &LowerName) -> Vec<LowerName> {
        self.config
            .ns_records
            .get(fqdn)
            .map_or(Vec::default(), Clone::clone)
    }

    fn a_rdata(&self, fqdn: &LowerName) -> Vec<RData> {
        self.addrs_from_config(fqdn)
            .iter()
            .filter_map(|ip| match ip {
                IpAddr::V4(ipv4_addr) => Some(RData::A(*ipv4_addr)),
                IpAddr::V6(_) => None,
            })
            .collect()
    }

    fn aaaa_rdata(&self, fqdn: &LowerName) -> Vec<RData> {
        self.addrs_from_config(fqdn)
            .iter()
            .filter_map(|ip| match ip {
                IpAddr::V4(_) => None,
                IpAddr::V6(ipv6_addr) => Some(RData::AAAA(*ipv6_addr)),
            })
            .collect()
    }

    fn ns_rdata(&self, fqdn: &LowerName) -> Vec<RData> {
        self.ns_names_from_config(fqdn)
            .iter()
            .map(|n| RData::NS(n.into()))
            .collect()
    }

    async fn send_auth_resp<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
        rdata: Vec<RData>,
    ) -> Result<ResponseInfo, Error> {
        let records: Vec<Record> = rdata
            .iter()
            .map(|rd| Record::from_rdata(request.query().name().into(), 1, rd.clone()))
            .collect();
        let mut header = Header::response_from_request(request.header());
        header.set_authoritative(true);
        let builder = MessageResponseBuilder::from_message_request(request);
        let response = builder.build(header, records.iter(), &[], &[], &[]);
        Ok(response_handle.send_response(response).await?)
    }

    async fn send_nxdomain<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> Result<ResponseInfo, Error> {
        let builder = MessageResponseBuilder::from_message_request(request);
        let mut header = Header::response_from_request(request.header());
        header.set_authoritative(true);
        header.set_response_code(ResponseCode::NXDomain);
        let response = builder.build_no_records(header);
        Ok(response_handle.send_response(response).await?)
    }
}

#[async_trait::async_trait]
impl RequestHandler for Handler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        response_handle: R,
    ) -> ResponseInfo {
        match self.dispatch_request(request, response_handle).await {
            Ok(info) => info,
            Err(error) => {
                error!("error in RequestHandler: {:?}", error);
                let mut header = Header::new();
                header.set_response_code(ResponseCode::ServFail);
                header.into()
            }
        }
    }
}
