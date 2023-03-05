{ config, lib, pkgs, ... }:
with lib;
let
  name = "acmecrab";
  cfg = config.services.${name};
  settingsFormat = pkgs.formats.json { };
in {
  # TODO(XXX): Additional validation checks/typing. E.g. of CIDR networks, FQDNs.
  options.services.${name} = {
    enable = mkEnableOption "Enable the ${name} service";

    domain = mkOption {
      type = types.str;
      example = "pki.example.com";
      description = ''
        Fully qualified domain name for the ACME Crab server.
        All TXT records must be subdomains of this FQDN.
      '';
    };

    ns_domain = mkOption {
      type = types.str;
      example = "ns1.pki.example.com";
      description = ''
        Fully qualified domain name for the nameserver to use in the SOA 
        record for options.services.${name}.domain.
      '';
    };

    ns_admin = mkOption {
      type = types.str;
      example = "dns-admin@example.com";
      description = ''
        Email address of the options.services.${name}.ns_domain
        administrator. Translated to record format (e.g.
        "foo@example.com" -> "foo.example.com") automatically.
      '';
    };

    # TODO(XXX): Make state optional.
    txt_store_state_path = mkOption {
      type = types.str;
      default = "/var/lib/${name}/txt_records.json";
      description = ''
        Path to a JSON data file for persisting TXT records across 
        shutdown. Created at startup if it does not exist.
      '';
    };

    api_addr = mkOption {
      type = types.str;
      example = "10.233.1.2";
      description = ''
        Bind address for HTTP API. Must be a loopback address or
        private network.
      '';
    };

    api_port = mkOption {
      type = types.numbers.positive;
      default = 8080;
      description = ''
        Port for the HTTP API on the api_addr bind address.
      '';
    };

    api_timeout = mkOption {
      type = types.numbers.positive;
      default = 120;
      description = ''
        Maximum duration for an API request before timing out, expressed 
        in seconds
      '';
    };

    dns_port = mkOption {
      type = types.numbers.positive;
      default = 53;
      description = ''
        Port for the HTTP API on the api_addr bind address.
      '';
    };

    dns_udp_addr = mkOption {
      type = types.str;
      default = "0.0.0.0";
      example = "10.233.1.2";
      description = ''
        Bind address for UDP DNS server.
      '';
    };

    dns_tcp_addr = mkOption {
      type = types.str;
      default = "0.0.0.0";
      example = "10.233.1.2";
      description = ''
        Bind address for TCP DNS server.
      '';
    };

    dns_tcp_timeout = mkOption {
      type = types.numbers.positive;
      default = 60;
      description = ''
        Maximum duration for a TCP DNS request before timing out, 
        expressed in seconds.
      '';
    };

    acl = mkOption {
      type = types.submodule {
        freeformType = types.attrsOf (types.listOf types.str);
      };
      default = { };
      example = { "127.0.0.0/24" = [ "subdomain_a" "subdomain_b" ]; };
      description = ''
        A map of CIDR networks and subdomains IPs within that network 
        can updated TXT records for.'';
    };

    addrs = mkOption {
      type = types.submodule {
        freeformType = types.attrsOf (types.listOf types.str);
      };
      default = { };
      example = {
        "pki.example.com" = [ "10.233.1.2" ];
        "ns1.pki.example.com" = [ "10.233.1.2" ];
      };
      description = ''
        A map of fully qualified domains and IP addresses that should
        be used for A/AAAA queries for each domain.
      '';
    };

    ns_records = mkOption {
      type = types.submodule {
        freeformType = types.attrsOf (types.listOf types.str);
      };
      default = { };
      example = { "pki.example.com" = [ "ns1.pki.example.com" ]; };
      description = ''
        A map of fully qualified domains to domain values that should be 
        returned for NS lookups. 
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.${name} = {
      wantedBy = [ "multi-user.target" ];
      # TODO(XXX): this is a decent start on hardening options but we can do better.
      serviceConfig = {
        Restart = "on-failure";
        ExecStart = "${pkgs.acmecrab}/bin/${name} /etc/${name}.json";
        Environment = "RUST_LOG=${name}=debug";
        DynamicUser = "yes";
        RuntimeDirectory = name;
        RuntimeDirectoryMode = "0755";
        StateDirectory = name;
        StateDirectoryMode = "0700";
        CacheDirectory = name;
        CacheDirectoryMode = "0750";
        AmbientCapabilities = "CAP_NET_BIND_SERVICE";
        CapabilityBoundingSet = "CAP_NET_BIND_SERVICE";
        ProtectHome = true;
        RestrictAddressFamilies = "AF_INET AF_INET6";
      };
    };

    networking.firewall = with cfg; {
      allowedTCPPorts = [ api_port dns_port ];
      allowedUDPPorts = [ dns_port ];
    };

    environment.etc."${name}.json".source = with cfg;
      settingsFormat.generate "${name}-config.json" {
        inherit domain ns_domain ns_admin txt_store_state_path api_timeout acl
          addrs ns_records dns_tcp_timeout;
        api_bind_addr = "${api_addr}:${toString api_port}";
        dns_udp_bind_addr = "${dns_udp_addr}:${toString dns_port}";
        dns_tcp_bind_addr = "${dns_tcp_addr}:${toString dns_port}";
      };
  };
}
