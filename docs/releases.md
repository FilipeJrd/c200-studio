# Releases

## Current status

The workspace version is **0.2.0**. There are no published GitHub Releases or
release tags yet. The existing workflows provide continuous validation and
development packages; they do not publish releases automatically.

- **CI** runs formatting, actionlint, Clippy, tests on Ubuntu 22.04/24.04,
  desktop entry validation, and Bash syntax checks.
- **AppImage** builds on Ubuntu 22.04 for the supported glibc baseline,
  opens the native GUI in demo mode, collects dependency license notices,
  packages the app, opens the AppImage without FUSE, and checks its SHA-256 sum.
- Both run for pull requests, pushes to `main`, and manual dispatch.
- AppImage artifacts contain the x86_64 package, checksum, and demo screenshots.
  They are retained for **14 days** and belong to the workflow's commit.
- There is no `v*` tag trigger, automatic changelog, release upload, or
  application update mechanism. Creating a tag alone does not run a release build.

## Download a development build

Open a successful run of the
[AppImage workflow](https://github.com/FilipeJrd/c200-studio/actions/workflows/appimage.yml)
and download `C200-Studio-linux-x86_64` from **Artifacts**. With GitHub CLI:

```sh
gh run list --repo FilipeJrd/c200-studio --workflow appimage.yml --branch main --status success
# Replace RUN_ID with the successful run for the commit you want.
gh run download RUN_ID --repo FilipeJrd/c200-studio --name C200-Studio-linux-x86_64 --dir dist/download
cd dist/download
sha256sum --check C200-Studio-x86_64.AppImage.sha256
chmod +x C200-Studio-x86_64.AppImage
./C200-Studio-x86_64.AppImage
```

Use `--appimage-extract-and-run` if FUSE is unavailable. Development artifacts
are temporary downloads; they should not be used as permanent release URLs.

## Manual versioned release

A maintainer can publish a release using the existing tooling:

1. Set the intended version in `[workspace.package]` in `Cargo.toml`, refresh
   `Cargo.lock`, record the changes and validation, and merge the reviewed change.
2. Wait for both workflows to pass for that exact `main` commit. Check the
   AppImage on the target desktop and real camera, and record material limitations.
3. Download the AppImage artifact from that commit's successful workflow run
   into `dist/release`. Check its checksum. Use that verified package rather
   than rebuilding on a different machine for upload.
4. Prepare English release notes and create a draft release targeting the
   same commit. Replace `COMMIT_SHA` and the release notes path below; use the
   version matching the workspace:

   ```sh
   gh release create v0.2.0 \
     dist/release/C200-Studio-x86_64.AppImage \
     dist/release/C200-Studio-x86_64.AppImage.sha256 \
     --repo FilipeJrd/c200-studio \
     --target COMMIT_SHA \
     --draft \
     --title 'C200 Studio 0.2.0' \
     --notes-file /path/to/release-notes.md
   ```

   This command creates the tag if it does not exist. Review the target commit,
   version, notes, asset names, and downloads in the draft before publishing it.
5. Publish the reviewed draft through GitHub Releases. Mark it as a prerelease
   if it is an alpha, beta, or release candidate.

## Suggested next automation

A dedicated release workflow could respond to tags such as `v0.2.1`, verify
that the tag matches the workspace version, run validation and packaging for
that commit, and upload the AppImage/checksum to a draft GitHub Release.
Publishing the reviewed draft would remain an explicit maintainer action.
Only the release upload job would need `contents: write`; PR checks would keep
their existing read-only permissions. This automation is not implemented yet.
