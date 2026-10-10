#!/usr/bin/env bash
set -euo pipefail
# Release signatures are checked by apt; snapshot timestamp and Release bytes are frozen.
rm -f /etc/apt/sources.list.d/debian.sources
printf '%s\n' \
  'deb [check-valid-until=no] https://snapshot.debian.org/archive/debian/20261009T000000Z bookworm main' \
  'deb [check-valid-until=no] https://snapshot.debian.org/archive/debian/20261009T000000Z bookworm-updates main' \
  'deb [check-valid-until=no] https://snapshot.debian.org/archive/debian-security/20261009T000000Z bookworm-security main' \
  > /etc/apt/sources.list
apt-get update
echo '77737fa4b34f2693e982cc9ee35736816c35a7778fc2d326cc1bbf5b301fe1aa  /var/lib/apt/lists/snapshot.debian.org_archive_debian_20261009T000000Z_dists_bookworm_InRelease' | sha256sum --check
echo '5aff6e6e76c43a3b3f8ed100aa1fdad5bb0dda38dd981ac6b92464947c23b2f6  /var/lib/apt/lists/snapshot.debian.org_archive_debian_20261009T000000Z_dists_bookworm-updates_InRelease' | sha256sum --check
echo '6192a790ed1d11bf7b14ccadfc4d8de3aacd98ea8c005299ed35bfd29bdf42c8  /var/lib/apt/lists/snapshot.debian.org_archive_debian-security_20261009T000000Z_dists_bookworm-security_InRelease' | sha256sum --check
