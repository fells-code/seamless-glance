---
'seamless-glance': patch
---

The `install.sh` and `install.ps1` installers now refuse to install a binary unless it matches its entry in the release's `SHA256SUMS.txt`. Before, `install.sh` installed without checking when the checksum file could not be downloaded, and `install.ps1` only warned, even on a checksum mismatch.
