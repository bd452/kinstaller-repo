# Kinstaller KPM Repository

Static [KPM](https://kindlemodding.org/kindle-dev/kpm/) repository for Kinstaller and related Kindle homebrew packages.

- **Manifest:** https://bd452.github.io/kinstaller-repo/manifest.json
- **Browse:** https://bd452.github.io/kinstaller-repo/

## Add on your Kindle

```text
;kpm add-repo https://bd452.github.io/kinstaller-repo/manifest.json
```

## Adding packages

1. Build a `.kpkg` with [kpm-helper.py](https://github.com/KindleModding/KPM/blob/main/kpm-helper.py):

   ```bash
   python kpm-helper.py package pack ./my-package ./output
   ```

2. Add it to this repository:

   ```bash
   python kpm-helper.py repo add . ./output/my-package_1.0.0_kindlehf-kindlepw2.kpkg
   ```

3. Commit and push. GitHub Actions builds the web index and publishes to GitHub Pages.

Package artifacts live under `packages/<package-id>/artifacts/`.
