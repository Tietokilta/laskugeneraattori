{ ... }:
{
  projectRootFile = "flake.nix";
  programs.nixfmt.enable = true;
  programs.rustfmt.enable = true;
  programs.taplo.enable = true;
}
