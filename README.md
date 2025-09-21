# one-block-army

**Adlist merger and whitelister**  
Created because of a small shortcoming in the adlist feature of RouterOS where you cannot whitelist hosts.



# Install

## General linux
- Clone the project `git clone git@github.com:rogierknoester/one-block-army.git`
- `cargo build --release`
- copy the binary to where you'd use it
- create a config file and point to it with the `--config ./config.toml` option

## NixOS

Include the `flake.nix` in your `flake`.

```nix

{
    inputs.one-block-army.url = "github:rogierknoester/one-block-army";
    inputs.one-block-army.inputs.nixpkgs.follows = "nixpkgs";

    outputs = {one-block-army, ...}: {
      nixosConfigurations = {
        server-a = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            one-block-army.nixosModules.one-block-army
          ];
        };
      };
    }
}

```

Enable the service with
```nix
services.one-block-army = {
  enable = true;
  listen = "0.0.0.0";
  port = 8000;
  adlists = [
    "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts"
    "https://blocklistproject.github.io/Lists/ads.txt"
    "https://raw.githubusercontent.com/hagezi/dns-blocklists/refs/heads/main/hosts/pro.plus.txt"
  ];

  whitelistedHosts = [
    "kagi.com"
    "sentry.io"
    "*.sentry.io" # globbing is supported
  ];
};
```

## Post install
Once its running you'll need to configure your DNS server or adblocking list to use it. For example on RouterOS:
```
/ip dns adlist add url=http://server-a-ip:8000
```

