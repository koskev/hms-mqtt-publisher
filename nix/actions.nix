{ inputs, ... }:
let
  inherit (inputs.nix-actions.lib) steps;
  inherit (inputs.nix-actions.lib) platforms;
  inherit (inputs.nix-actions.lib) mkCachixSteps;
  actions = inputs.nix-actions.lib.actions // {
    setup-helm = "azure/setup-helm@dda3372f752e03dde6b3237bc9431cdc2f7a02a2"; # v5.0.0
    chart-releaser = "helm/chart-releaser-action@cae68fefc6b5f367a0275617c9f83181ba54714f"; # v1.7.0
  };
in
{
  imports = [ inputs.actions-nix.flakeModules.default ];
  flake.actions-nix = {
    pre-commit.enable = true;
    defaultValues = {
      jobs = {
        runs-on = "ubuntu-latest";
      };
    };
    workflows = {
      ".github/workflows/docker-publish.yaml" = inputs.nix-actions.lib.mkDocker {
        targetPlatforms = [
          platforms.linux
          platforms.linux_aarch64
          platforms.mac
        ];
      };
      ".github/workflows/linting.yaml" = inputs.nix-actions.lib.mkClippy { };
      ".github/workflows/test.yaml" = {
        on = {
          push = { };
          pull_request = { };
        };
        env = {
          CARGO_TERM_COLOR = "always";
        };
        jobs = {
          nix-build = {
            strategy.matrix.platform = [
              platforms.linux
              platforms.linux_aarch64
              platforms.mac
            ];
            runs-on = "\${{ matrix.platform.runs-on }}";
            steps = [
              steps.checkout
              steps.installNix
              {
                name = "Build";
                run = "nix build .";
              }
            ]
            ++ mkCachixSteps { };
          };
        };
      };
      ".github/workflows/chart.yaml" = {
        name = "Release Charts";
        on.push.tags = [ "*" ];
        jobs.release = {
          permissions.contents = "write";
          steps = [
            steps.checkout-full
            {
              name = "Configure Git";
              run = ''
                git config user.name "$GITHUB_ACTOR"
                git config user.email "$GITHUB_ACTOR@users.noreply.github.com"
              '';
            }
            {
              name = "Install Helm";
              uses = actions.setup-helm;
            }
            {
              name = "Run chart-releaser";
              uses = actions.chart-releaser;
              env.CR_TOKEN = "\${{ secrets.GITHUB_TOKEN }}";
            }
          ];
        };
      };
    };
  };
}
