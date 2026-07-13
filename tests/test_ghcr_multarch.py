from scripts.verify_ghcr_multarch import REQUIRED_PLATFORMS, parse_inspect_output


def test_parse_inspect_output_finds_required_platforms():
    output = "Name: ghcr.io/example/fyi-mcp:v0.1.2\nlinux/amd64\nlinux/arm64\n"
    assert parse_inspect_output(output) == REQUIRED_PLATFORMS


def test_parse_inspect_output_does_not_infer_missing_platforms():
    assert parse_inspect_output("linux/amd64\n") == {"linux/amd64"}
