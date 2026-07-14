#!/usr/bin/env python3
"""Download every pinned artifact and verify its declared size and digest."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen


REPO_ROOT = Path(__file__).resolve().parent.parent
USER_AGENT = "bd452-kpm-repo-artifact-verifier/1"
DOWNLOAD_TIMEOUT_SECONDS = 120


def verify_remote_artifact(url: str, expected_size: int, expected_sha256: str) -> None:
    parsed = urlparse(url)
    if parsed.scheme != "https" or not parsed.netloc:
        raise SystemExit(f"artifact URL must be an absolute HTTPS URL: {url}")

    digest = hashlib.sha256()
    actual_size = 0
    request = Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urlopen(request, timeout=DOWNLOAD_TIMEOUT_SECONDS) as response:
            final_url = response.geturl()
            if urlparse(final_url).scheme != "https":
                raise SystemExit(f"artifact download redirected away from HTTPS: {url}")
            for chunk in iter(lambda: response.read(1024 * 1024), b""):
                actual_size += len(chunk)
                digest.update(chunk)
    except (HTTPError, URLError, TimeoutError) as error:
        raise SystemExit(f"failed to download {url}: {error}") from error

    if actual_size != expected_size:
        raise SystemExit(
            f"size mismatch for {url}: expected {expected_size}, got {actual_size}"
        )
    actual_sha256 = digest.hexdigest()
    if actual_sha256 != expected_sha256:
        raise SystemExit(
            f"SHA-256 mismatch for {url}: expected {expected_sha256}, got {actual_sha256}"
        )


def main() -> None:
    checked = 0
    for descriptor_path in sorted((REPO_ROOT / "registry" / "sources").glob("*.json")):
        descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
        for package in descriptor.get("packages", []):
            artifact = package["artifact"]
            verify_remote_artifact(
                artifact["url"], artifact["size"], artifact["sha256"]
            )
            print(f"Verified {package['id']} {package['version']}: {artifact['url']}")
            checked += 1
    if checked == 0:
        raise SystemExit("no artifacts found in registry release descriptors")
    print(f"Verified {checked} remote artifact(s).")


if __name__ == "__main__":
    main()
