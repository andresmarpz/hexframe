# Release signing

Releases are signed for free by the SignPath Foundation open-source programme. Windows shows "SignPath Foundation" as the publisher. The website's code signing policy page (`/code-signing/`) states the terms SignPath asks for.

## How a release runs

1. `just release vX.Y.Z` checks the tree, the version and the changelog entry, runs `just check`, then pushes the tag.
2. The `Release` workflow (`.github/workflows/release.yml`) builds `greyscreen.exe` and `greyscreen-cli.exe` on a GitHub-hosted Windows runner. SignPath only signs builds made on GitHub-hosted runners, so releases no longer build on the Mac. The repository is public, so these runners cost nothing.
3. The workflow uploads both files as the `unsigned` artifact and sends a signing request with the `release-signing` policy. Andres approves it in SignPath. The job waits up to an hour.
4. The `publish` job creates the GitHub release with the signed files, `GreyscreenSetup.exe` (a copy of `greyscreen.exe`), `README.txt` and `LICENSE.txt`, then calls the Vercel deploy hook.

A manual run of the workflow (`workflow_dispatch`) builds and signs with the `test-signing` policy and publishes nothing. Use it to check the pipeline.

Until the repository variable `SIGNPATH_ORGANIZATION_ID` exists, the workflow skips signing and publishes unsigned files.

## SignPath setup

Rules from https://signpath.org/terms that matter here:

- OSI license for all components (AGPL-3.0; Rune is MIT), no proprietary parts.
- MFA on GitHub and SignPath for every team member.
- Every signing request approved by hand. Every change from outside the team reviewed.
- The privacy policy is linked from the setup window and update checks can be turned off.
- An uninstaller exists (Windows Settings > Apps, `--uninstall`).
- Product name and version metadata enforced on the signed files. Both binaries carry `ProductName` "Greyscreen" and the workspace version (`winresource` in each `build.rs`).

Steps after SignPath accepts the application:

1. In SignPath, add the predefined "GitHub.com" trusted build system to the organization and link it to the `greyscreen` project. Install the SignPath GitHub App on the repository.
2. Set the project's artifact configuration to the XML below.
3. Create a CI user with submitter rights on both signing policies and copy its API token.
4. In GitHub repository settings, add the secret `SIGNPATH_API_TOKEN` and the variable `SIGNPATH_ORGANIZATION_ID`. Add `SIGNPATH_PROJECT_SLUG` only if the project slug is not `greyscreen`.
5. Run the workflow by hand and check that both files come back signed with the test certificate.

Artifact configuration:

```xml
<?xml version="1.0" encoding="utf-8"?>
<artifact-configuration xmlns="http://signpath.io/artifact-configuration/v1">
  <parameters>
    <parameter name="version" required="true" />
  </parameters>
  <zip-file>
    <pe-file path="greyscreen.exe" product-name="Greyscreen" product-version="${version}">
      <authenticode-sign />
    </pe-file>
    <pe-file path="greyscreen-cli.exe" product-name="Greyscreen" product-version="${version}">
      <authenticode-sign />
    </pe-file>
  </zip-file>
</artifact-configuration>
```

The workflow passes `version` from the workspace `Cargo.toml`.
