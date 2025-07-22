{
  pkgs,
  lib,
  config,
  ...
}:
let
  cfg = config.services.one-block-army;

  settingsFormat = pkgs.formats.toml { };

  settingsFile = settingsFormat.generate "one-block-army.toml" {
    adlists = cfg.adlists;
    "whitelisted_hosts" = cfg.whitelistedHosts;
  };

in
{
  options.services.one-block-army = {
    enable = lib.mkEnableOption "one-block-army service";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.callPackage ./package.nix { };
      description = "The one-block-army package to use";
    };

    listen = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "Address to listen on";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8000;
      description = "Port to listen on";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "one-block-army";
      description = "User to run the service with";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "one-block-army";
      description = "Group to run the service with";
    };

    adlists = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      description = "The adlists to use";
    };

    whitelistedHosts = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      description = "Hosts that should be whitelisted";
    };

  };

  config = lib.mkIf cfg.enable {

    systemd.services.one-block-army = {
      description = "one-block-army";
      requires = [ "network-online.target" ];
      after = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${lib.getExe cfg.package} --config ${settingsFile} --port ${builtins.toString cfg.port} --listen ${cfg.listen}";
        Restart = "always";
        User = cfg.user;
        Group = cfg.group;
      };
    };

    users.users.${cfg.user} = {
      description = "one-block-army user";
      group = cfg.group;
      isSystemUser = true;
    };

    users.groups.${cfg.group} = { };
  };
}
