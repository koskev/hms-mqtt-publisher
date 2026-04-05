_: {
  perSystem =
    {
      inputs',
      self',
      ...
    }:
    let
      nix2containerPkgs = inputs'.nix2container.packages;
    in
    {
      packages = {
        dockerImageFull = nix2containerPkgs.nix2container.buildImage {
          name = "hms-mqtt-publisher";
          tag = "latest";

          config = {
            Cmd = [ "${self'.packages.default}/bin/hms-mqtt-publish" ];
            Env = [
              "PATH=${self'.packages.default}/bin"
            ];
          };
        };
      };
    };
}
