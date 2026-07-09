# SignalKit documentation

Guides for the `signalkit` crate. Start with [concepts](concepts.md) if you are
new to the reactive model, then [usage](usage.md) to build a screen.

| Guide | When to read it |
|-------|-----------------|
| [concepts.md](concepts.md) | How signals, components, and the retained tree fit together |
| [usage.md](usage.md) | API walkthrough: widgets, layout, `slot`, `for_each`, bindings |
| [device.md](device.md) | On-device rendering, refresh policy, touch, `App::run` |
| [c-abi.md](c-abi.md) | Calling SignalKit from C (or any language that can load a `.so`) |
| [building.md](building.md) | Cold-start build/dev: host tests, koxtoolchain, Linux + macOS/Docker cross builds, deploy, troubleshooting |

The crate README ([../README.md](../README.md)) has a short overview and quick
start. The workspace README ([../../README.md](../../README.md)) covers the
Cargo workspace and KPM packages.
