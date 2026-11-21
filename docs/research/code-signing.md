# Code signing greyscreen on Windows (researched 2026-11-21)

Goal: an individual open-source developer in Uruguay signs a Rust exe (cross-compiled from macOS with cargo-xwin, released from GitHub Actions) so users see a publisher name instead of "Unknown publisher", and see as little of SmartScreen's "Windows protected your PC" as possible.

Legend: **[V]** verified on a primary source (Microsoft Learn, vendor page) fetched today. **[S]** secondary source (reseller, forum, community post). **[U]** could not verify.

## Ground truth about SmartScreen in 2026

- SmartScreen checks two signals: publisher reputation (is the file signed by a known certificate) and file-hash reputation. A new signed binary can still warn until hash or publisher reputation builds. [V] [SmartScreen reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation) (page dated 2026-05, updated 2026-08).
- No certificate type gives instant reputation. Microsoft's table: Store = no warning; valid OV/EV = warning "until reputation accumulates", publisher name shown; unsigned or self-signed = "Windows protected your PC". [V]
- EV no longer bypasses SmartScreen: "EV certificates no longer bypass SmartScreen ... Paying a premium for EV solely to avoid SmartScreen warnings is no longer justified." [V] Same page, plus [code signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options). The 2024 change removed the EV OIDs from Trusted Root Program roots (August 2024) [S: Microsoft Q&A quoting the program requirements].
- Reputation builds over "several weeks and hundreds of clean installs from a wide audience"; there is no threshold and no manual submission for consumers. [V]
- Reputation carries across releases only if they are signed with the same publisher identity. Changing certificate changes the publisher signal. Unsigned files restart at zero on every version. [V] A renewal's carry-over is not documented. [S: DigiCert KB / Microsoft Q&A; [U] officially]
- Windows 11 Smart App Control blocks unsigned files without positive reputation, for every executable, not only downloaded ones. Signing helps here too. [V]
- Since 2023-06-01 CA/B Forum rules require code signing keys in a hardware module (token or cloud HSM). Public certs are capped at 459 days from late February 2026 (Certum and SSL.com both state this). So "buy a PFX and put it in a GitHub secret" no longer exists for public CAs. [V] [CSBR](https://cabforum.org/working-groups/code-signing/requirements/); [V] Certum shop banner for the 459-day cap.
- Signature timestamping (RFC 3161) is needed so signatures outlive the 459-day certificate.

## 1. Azure Artifact Signing (formerly Trusted Signing)

Verdict: **not available to this developer.**

- Eligibility: [V] Quickstart prerequisites: "Public Trust certificates are available to organizations in the United States, Canada, the European Union, the United Kingdom, Australia, New Zealand, Japan, South Korea, Singapore, Switzerland, Norway, and Israel. Individual developers must be located in the United States or Canada." ([quickstart](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart), updated 2026-11-20.) Uruguay is on neither list. Private Trust has no geographic limit but is not publicly trusted, so it does not remove the warning.
- Workaround "make a company elsewhere" would need a legal entity in a listed country. Out of scope for a hobby project.
- Cost: Basic $9.99/month, 5,000 signatures; Premium 100,000 signatures. [V] $9.99 from Microsoft docs; the Azure pricing page did not render amounts [U for Premium price]. Needs a paid Azure subscription (free/trial/sponsored rejected). [V FAQ]
- Identity validation (for US/Canada individuals): billing account of type Individual, government ID plus selfie via AU10TIX and Microsoft Authenticator Verified ID; org validation takes 1 to 30 business days. [V]
- CI: works from GitHub Actions without a token; certs are short-lived and never exposed. Jsign, SignTool, and the official action work. [V]
- SmartScreen: reputation builds over time; not instant. [V] Not EV: "no plan to issue EV certificates". [V]
- Renewal: automatic while identity validation is valid; validation must be renewed or signing stops. [V]

## 2. Certum "Open Source Code Signing in the Cloud" (SimplySign)

Verdict: **cheapest paid route that an individual in Uruguay can plausibly get today.**

- Price: **EUR 49.00** (cloud, SimplySign). Card variant "Open Source Code Signing - set" EUR 69.00 (card plus reader, non-returnable card). "Open Source Code Signing - code" EUR 25.00 (for people who already own a card, presumably the renewal route [U]). [V] [Certum shop](https://shop.certum.eu/code-signing.html), [cloud product](https://shop.certum.eu/open-source-code-signing-on-simplysign.html), [set product](https://shop.certum.eu/open-source-code-signing.html).
- Validity and renewal price: not stated on the product pages. Max is 459 days since February 2026. [U for exact term; V for the cap]. Assume about EUR 25 to 49 per year; confirm at checkout.
- Eligibility: [V] [required documents](https://support.certum.eu/en/code-signing-required-documents/)
  - Individuals only. Subject becomes "CN = Open Source Developer, <your name>", "O = Open Source Developer".
  - Identity: automatic online verification (photos of ID card, passport or licence, face compared), or notary, or Registration Point.
  - A utility bill in your name.
  - The URL of an ongoing, **publicly available** open-source project that clearly shows your relationship to it. "If Certum cannot identify the project from public information, the certificate will not be issued." So the repo must be public first.
  - Revoked if used to sign commercially distributed software. Free MIT recorder is fine.
  - Non-EU applicants: no separate rules on the page; whether a Uruguayan ID and utility bill are accepted is [U]. Certum says to ask support.
- Hardware: no token. Key lives in Certum's cloud HSM, used through SimplySign Desktop (virtual card) plus a mobile app that makes access codes (TOTP). Limit 5,000 signatures per month. Keys are RSA 3072+. [V]
- Scripting in CI: Certum's own manual covers only Windows SimplySign Desktop plus signtool. [V] No official headless route found. [U for "none exists"]
  - Community tool [ssign](https://docs.rs/ssign) (MIT, Rust, "not affiliated with or endorsed by Certum"): logs in over HTTPS with account email plus the TOTP seed or a current 6 digit code, signs PE files (.exe, .dll) from Linux, macOS or Windows, adds an RFC 3161 timestamp from time.certum.pl. Includes a GitHub Actions example using a protected `signing` environment. [V] Limits: PE only (MSI not enabled, no MSIX/CAB); cannot verify; "only works with Certum SimplySign cloud certificates". It does not state that Open Source certificates work [U: test needed before paying for CI work].
  - `ssign-pkcs11` module lets osslsigncode sign via the same session. [V from osslsigncode README]
  - Risk: the TOTP seed is a long-lived signing credential; anyone with seed plus email can sign as you until you re-issue the SimplySign QR. Keep it in a GitHub Environment secret with required reviewer, never on fork PRs. [V from ssign docs]
  - Alternative with no seed in CI: build in CI, download, sign on the Mac with a fresh 6 digit code from the phone, upload to the release. One manual step per release.
- SmartScreen: Certum claims its certs help build reputation [S: vendor marketing]. Treated by Windows like any OV cert: warning at first, then builds. No source for how fast an individual "Open Source Developer" identity earns it [U]. Same publisher identity must be kept across releases.
- Renewal: reissue yearly (about), validation likely repeated; carry-over of reputation to a renewed cert [U].
- Works from macOS or GitHub Actions: yes via ssign or osslsigncode+ssign-pkcs11; not via signtool on Linux/macOS.

## 3. SignPath Foundation (free for OSS)

Verdict: **free and good fit if approved; approval is discretionary and wants a public, released, maintained project.**

From [signpath.org/terms](https://signpath.org/terms) (page is marked draft) [V]:
- OSI-approved license for all components (MIT ok), no proprietary components, no malware or PUPs.
- Actively maintained and **already released** in the form to be signed; functionality described on the download page or app-store entry.
- Source in a public repo, built by an automated pipeline, binaries "built from source in a verifiable way" (GitHub Actions with the SignPath action is the supported pattern; Jsign also has a `SIGNPATH` store type [V]).
- Signing team = development team and repo owner. Define Authors, Reviewers, Approvers (one person may hold all roles for a solo project [U]). MFA on SignPath and on the repo for all members.
- **Every release needs manual approval** in SignPath.
- Product name must equal project name; same version string across builds (metadata restrictions enforced).
- Privacy statement if data is sent anywhere; installer must offer uninstall; system changes need warnings.
- Website must carry a "Code signing policy" section crediting "Free code signing provided by SignPath.io, certificate by SignPath Foundation", with team roles and privacy policy.
- Process: apply at signpath.org/apply; initial reputation and control check; acceptance at SignPath's discretion with no arbitration. No timeline published [U].
- The publisher shown to users is **SignPath Foundation**, not the developer. Reputation is shared with every other Foundation-signed project's certificate [V that cert is issued to the Foundation; effect on SmartScreen [U]].
- Cost: EUR 0. Hardware token: none, cloud HSM. Works from GitHub Actions (and so independent of the macOS build host). cargo-xwin output can be uploaded as an artifact to SignPath. [V for the pattern; [U] not tested]
- Microsoft's own docs list it as the OSS option, "OV-level certificate signing through a managed pipeline". [V] [code signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
- Likelihood of approval for a brand-new project with no releases and no community: [U]. Applying after the first public releases is safer.

## 4. Classic OV and EV certificates

Verdict: **works, costs more, no SmartScreen advantage.**

- Microsoft's guide: OV about USD 150 to 300 per year; EV USD 400+. [V] Reseller prices [S]: Sectigo OV/IV about USD 220 to 279 per year; DigiCert OV about USD 439 per year. DigiCert KeyLocker cloud price not found [U].
- SSL.com eSigner (cloud HSM, REST API, CI-friendly): IV/OV tier 1 = USD 20/month or USD 180/year for 20 signings per month, extra USD 1 each; EV tier 1 = USD 100/month or USD 900/year for 10 signings. First 30 days free. [V] [eSigner pricing](https://www.ssl.com/guide/esigner-pricing-for-code-signing/). The price of the certificate itself on top of eSigner is not shown on that page [U]. SSL.com offers Individual Validated (IV) and Sole Proprietor EV for people with no company. [V] [ssl.com/code-signing](https://www.ssl.com/code-signing/) Acceptance of a Uruguayan individual: [U].
- Sectigo "individual" certs are still sold to individuals [S]. Country acceptance [U].
- EV: needs hardware (YubiKey FIPS token) or eSigner/KeyLocker; org-grade validation; no instant reputation since 2024. [V]
- Jsign supports `ESIGNER` (SSL.com) and `DIGICERTONE` (DigiCert ONE/KeyLocker) stores, so macOS or Linux CI works. [V]
- Tokens shipped to Uruguay add customs friction [U]; cloud options avoid that.
- Renewal: yearly, new validation each time at most CAs [U].

## 5. Microsoft Store distribution

Verdict: **the only route with zero SmartScreen warning, but it changes the app's packaging and update model.**

- Account: individual developer accounts have **no registration fee** now (the $19 is gone), via the new flow at storedeveloper.microsoft.com only; other entry points show the legacy flow. Company accounts are also free (announced May 2026). ID plus selfie verification, personal Microsoft account. [V] [open an account](https://learn.microsoft.com/en-us/windows/apps/publish/partner-center/open-a-developer-account), [S] [Windows blog 2025-09](https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/) ("over 200 markets"). Whether Uruguay is a supported registration country: [U]; I could not load the market list.
- Signing:
  - **MSIX package**: Microsoft re-signs it after certification. No certificate to buy. Users never see SmartScreen for Store installs. [V]
  - **EXE/MSI submission** (allowed since June 2021): Microsoft does **not** re-sign; installer and all PE files need your own Authenticode cert chaining to a Trusted Root Program CA. Needs silent install, offline standalone installer, versioned immutable URL, no Store auto-update. [V] [MSI/EXE requirements](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/app-package-requirements). This path does not remove the need for sections 2 to 4.
- Capabilities for a full-trust MSIX (Win32 exe packaged with `uap10:TrustLevel="mediumIL"`):
  - `runFullTrust` is required, and is a **restricted capability**: you justify it in Partner Center during submission. [V] [capabilities](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations)
  - Full-trust apps run at medium integrity, not in an AppContainer. `internetClient` and similar network capabilities "apply only to AppContainer apps"; a Medium IL app "can already perform those operations". So a localhost HTTP client needs no network capability in full trust. [V] Whether a full-trust MSIX app is still subject to any loopback restriction: Microsoft's text implies none, but I found no explicit statement [U]; test it. If someone later set the app to AppContainer, loopback would be blocked by default and `internetClient`/`privateNetworkClientServer` do not cover loopback [S; U].
  - Windows.Graphics.Capture: `graphicsCapture` is only required for `GraphicsCapturePicker`; `graphicsCaptureProgrammatic` is only for creating items from WindowId/DisplayId; `graphicsCaptureWithoutBorder` for `IsBorderRequired = false` (needed to hide the yellow border). [V] `IGraphicsCaptureItemInterop::CreateForWindow/CreateForMonitor` from an HWND is the Win32 path (Windows 10 1903+). [V] Whether the interop path needs any manifest capability in a packaged app: [U]; test it. Recommend declaring `graphicsCaptureWithoutBorder` if the app turns the border off [V that it is the matching capability].
  - WASAPI process loopback (`ActivateAudioInterfaceAsync` with process-loopback params, Windows 10 build 20348 or newer): Microsoft says activation "may display a consent prompt ... depending on which WASAPI interface is activated" (microphone example). Whether process loopback triggers it or needs the `microphone` capability in a package: [U]. Test on a packaged build. Declaring `microphone` would show a privacy-sensitive capability in the listing and gate the app on the Windows mic privacy toggle [V that privacy-sensitive capabilities respect user toggle].
- Other costs: self-updater must go (Store-distributed MSIX updates through the Store; I found no full-trust-specific policy text, but community answers say a custom updater is not supported) [S/U]; Store certification review (days) per release; policy review of a game-recording tool near anti-cheat software [U]; cargo-xwin on macOS cannot run `MakeAppx`/`SignTool`; an MSIX can be built with the cross-platform `makemsix` from msix-packaging or on windows-latest in CI [U not tested].
- SmartScreen: none for Store installs. Direct downloads from GitHub keep the old behavior. [V]
- Renewal: nothing. [V]

## 6. winget and Scoop

Verdict: **help distribution and install flow, not the signature question.**

- winget manifests only need an installer URL, SHA256, silent-install support and metadata; the Microsoft doc lists no signing requirement. [V] [manifest doc](https://learn.microsoft.com/en-us/windows/package-manager/package/manifest) ("All tools must support a silent install"). The winget-pkgs repo's moderation and automated validation (including Defender scan of the installer) is described elsewhere and I did not verify whether an unsigned installer passes [U]. The suspicion "winget requires a signed installer" is not supported by the manifest docs; it is neither confirmed nor refuted for the community repository policy [U].
- Installs through winget or Scoop download inside the tool, so the file carries no Mark-of-the-Web and the browser SmartScreen prompt generally does not appear. [S: West Wind blog 2026-06 and others]; Microsoft says the Mark of the Web is not the only factor [S]. Smart App Control can still judge an unsigned exe at run time. [V]
- Scoop: not researched against a primary source. A bucket entry needs URL plus hash; MOTW behavior as above [S/U].
- Practical: publish to winget (and a Scoop bucket) after signing. They improve the user path but do not replace a certificate; users who download from the GitHub Releases page still see the warning.
- The manifest `Publisher` should match the Add/Remove Programs entry. [V]

## 7. Tools that sign from macOS or Linux

| Tool | Type | Providers it works with | Notes |
|---|---|---|---|
| [osslsigncode](https://github.com/mtrojnar/osslsigncode) | C, OpenSSL | PKCS#11 (hardware tokens, SimplySign via third-party `ssign-pkcs11`), PFX files. No native Azure/eSigner/KeyLocker support | Signs PE, CAB, CAT, MSI, APPX (so MSIX) and scripts; Linux/macOS (brew). Needs OpenSSL 3 for the provider route. [V] |
| [Jsign](https://ebourg.github.io/jsign/) | Java | Azure Artifact Signing (`TRUSTEDSIGNING`), SSL.com `ESIGNER`, DigiCert ONE `DIGICERTONE`, Google KMS, AWS KMS, `SIGNPATH`, generic `PKCS11`, PFX/JKS | EXE, DLL, MSI, MSIX/APPX, CAB, scripts. Linux/macOS/Windows, GitHub Action exists. [V] No Certum SimplySign store [U: not listed]. |
| [ssign](https://docs.rs/ssign) | Rust | Certum SimplySign cloud only | PE only today. Third-party. [V] |
| rcodesign (apple-codesign) | Rust | n/a | Apple platforms only; no Authenticode signing found. Not usable here. [S] |
| signtool.exe (+ vendor dlib) | Windows | Azure Artifact Signing, SimplySign Desktop, eSigner CKA, KeyLocker | Needs windows-latest runner; the usual route in GitHub Actions. [V for Azure/Certum manual] |

Signing must happen after the final exe is written (do not post-process a signed file). With cargo-xwin the exe can be signed on macOS (Jsign/osslsigncode) or on a windows-latest job; both work.

## Comparison

| Option | Cost | Uruguay individual? | Identity steps | Token/cloud | CI from GitHub Actions / macOS | SmartScreen | Renewal |
|---|---|---|---|---|---|---|---|
| Azure Artifact Signing | $9.99/mo+ | **No** (US/CA only; Uruguay not an org country either) | ID+selfie via Verified ID | Cloud | Yes / yes (Jsign) | Builds with downloads | auto while validation valid |
| Certum Open Source (cloud) | EUR 49 | Likely yes [U for Uruguay docs] | ID photos or notary, utility bill, public OSS project | SimplySign cloud + phone TOTP | Via third-party ssign (+osslsigncode); official route is Windows desktop | Builds with downloads; shows "Open Source Developer" | about yearly (<=459 days), term/price [U] |
| SignPath Foundation | free | Likely yes [U] (no country rule on terms page) | project review, MFA, public repo, released | Cloud | Yes (official GitHub action) | Builds; publisher is SignPath Foundation | none stated; ongoing policy compliance |
| OV (Sectigo/DigiCert/SSL.com IV) | about $150-440/yr; eSigner $180+/yr | [U] per CA | CA validation | Token or cloud HSM | Via eSigner/KeyLocker (Jsign) | Builds, same as OV | yearly |
| EV | $400+/yr (eSigner EV $900/yr) | [U] | org-grade | Token or cloud | as above | **Same as OV** | yearly |
| Store MSIX | free account | Market list [U] | ID+selfie | Microsoft re-signs | Submission via Partner Center | **No warning** for Store installs | none |
| winget/Scoop | free | n/a | none | n/a | n/a | Avoids browser MOTW prompt, not a signature | n/a |

## Recommendation

Ranked for this exact case (individual in Uruguay, free MIT app, GitHub Releases, macOS cross-build, repo about to go public):

1. **Certum Open Source Code Signing in the Cloud, EUR 49 (about USD 55) per year.** The only paid route that is available to a non-EU individual, requires no company, no token to ship, and no money beyond a small yearly fee. Plan:
   - Make the repo public and put the project URL in the Certum application (required).
   - Buy it, pass the ID and utility bill check.
   - Before wiring CI, test once locally: SimplySign cloud cert + `ssign` signs a greyscreen.exe from the Mac, and `signtool verify /pa` on a Windows box shows "Open Source Developer, <name>".
   - Then either sign in a GitHub Environment-protected job with `ssign` (seed stored as secret, reviewer required), or sign by hand per release.
   - Always timestamp. Keep the same certificate subject across releases so reputation accumulates.
   - Expect SmartScreen to still warn for the first weeks; the warning changes from "Unknown publisher" to the publisher name.
2. **SignPath Foundation (free).** Apply right after the first public release exists and the README has a Code signing policy section. Better pipeline security (key in their HSM, manual approval gate per release, official GitHub integration) and zero cost, but acceptance is discretionary and users will see "SignPath Foundation" as publisher. Use as the free fallback if Certum rejects the identity documents, or as the long-term choice if approved. Do not do both for the same app: switching publisher resets the publisher reputation.
3. **Microsoft Store (MSIX), after 1 or 2.** Free, and the only way to guarantee no SmartScreen warning for those who install from the Store. Costs a packaging project (MSIX build, `runFullTrust` justification, removing the self-updater, testing WGC, process loopback and localhost under the package). Treat it as an additional channel, not a replacement for signing the GitHub exe. Check first that Uruguay is a supported registration market [U].

Not recommended: Azure Artifact Signing (ineligible), EV (no benefit since 2024), OV from DigiCert/Sectigo (several times the cost for the same SmartScreen result).

What the developer must do personally:
- Make the repo public, release once, and publish a page that names the project and him as maintainer.
- Complete Certum's ID verification (ID photos or notary, utility bill) and pay EUR 49; install the SimplySign mobile app and capture the TOTP seed/QR safely.
- Create GitHub Environment secrets and approve the first signing run (or sign locally).
- Optionally apply to SignPath Foundation and open a Store developer account at storedeveloper.microsoft.com.

Not verified and worth testing before committing: Certum acceptance of a Uruguayan ID and utility bill; Certum Open Source certs working with ssign; certificate term and renewal price; Uruguay in the Store registration market list; Store behaviors for WGC border, process loopback consent and localhost under MSIX; whether winget-pkgs accepts an unsigned installer.
