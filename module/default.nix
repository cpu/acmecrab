{ self, config, lib, pkgs, ... }:
with lib;
let
  cfg = config.services.acmecrab;
  settingsFormat = pkgs.formats.json { };
in {
  # TODO(XXX): Additional validation checks/typing. E.g. of CIDR networks, FQDNs.
  options.services.acmecrab = {
    enable = mkEnableOption "Enable the acmecrab service";

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
        record for options.services.acmecrab.domain.
      '';
    };

    ns_admin = mkOption {
      type = types.str;
      example = "dns-admin@example.com";
      description = ''
        Email address of the options.services.acmecrab.ns_domain
        administrator. Translated to record format (e.g.
        "foo@example.com" -> "foo.example.com") automatically.
      '';
    };

    # TODO(XXX): Make state optional.
    txt_store_state_path = mkOption {
      type = types.str;
      default = "/var/lib/acmecrab/txt_records.json";
      description = ''
        Path to a JSON data file for persisting TXT records across 
        shutdown. Created at startup if it does not exist.
      '';
    };

    api_bind_addr = mkOption {
      type = types.str;
      example = "10.233.1.2:8080";
      description = ''
        Bind address for HTTP API. Must be a loopback address or
        private network.
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

    dns_udp_bind_addr = mkOption {
      type = types.str;
      default = "0.0.0.0:53";
      example = "10.233.1.2:53";
      description = ''
        Bind address for UDP DNS server.
      '';
    };

    dns_tcp_bind_addr = mkOption {
      type = types.str;
      default = "0.0.0.0:53";
      example = "10.233.1.2:53";
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
    systemd.services.acmecrab = {
      wantedBy = [ "multi-user.target" ];
      serviceConfig = let
        pkg = self.packages.${pkgs.system}.acmecrab;
        # TODO(XXX): this is a decent start on hardening options but we can do better.
      in {
        Restart = "on-failure";
        ExecStart = "${pkg}/bin/acmecrab /etc/acmecrab.json";
        Environment = "RUST_LOG=acmecrab=debug";
        DynamicUser = "yes";
        RuntimeDirectory = "acmecrab";
        RuntimeDirectoryMode = "0755";
        StateDirectory = "acmecrab";
        StateDirectoryMode = "0700";
        CacheDirectory = "acmecrab";
        CacheDirectoryMode = "0750";
        AmbientCapabilities = "CAP_NET_BIND_SERVICE";
        CapabilityBoundingSet = "CAP_NET_BIND_SERVICE";
        ProtectHome = true;
        RestrictAddressFamilies = "AF_INET AF_INET6";
      };
    };

    environment.etc."acmecrab.json".source =
      settingsFormat.generate "acmecrab-config.json" cfg;
  };
}
