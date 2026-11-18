# Licence notice — Gateways to Eternity (Horizon port)

The mod ported here is **Gateways to Eternity** by Brennan Ward
(`Shadows_of_Fire`), MIT.

- Where the licence was read: the ATM10 jar's own manifest,
  `META-INF/neoforge.mods.toml`, field `license="MIT License"`
  (`GatewaysToEternity-1.21.1-5.1.0.jar`, `modId="gateways"`,
  `version="5.1.0"`), and the upstream repository's `LICENSE` file
  (`https://github.com/Shadows-of-Fire/GatewaysToEternity`, MIT, "Copyright (c)
  2020 Brennan Ward").
- What travels from upstream in this port: the item texture
  `assets/gateways/textures/item/gate_pearl_small.png` and the language keys in
  `assets/gateways/lang/en_us.json`, both copied from the jar; the wave numbers
  and reward tables are carried from the jar's `data/gateways/gateways/*.json`.
- What is new: the Rust rewrite (`mixins/gateways/`) and the 26.3 content
  declarations (`data/gateways/horizon/*`, the recipe and the item definition).
  No Java source is copied or translated; the behaviour was read from the
  upstream repository for the rewrite only.
- The full MIT text is `LICENSE` in this repository and inside the delivered
  `.hnmod` (`mixins/gateways/LICENSE`), with this notice as
  `mixins/gateways/NOTICE.md`; the built `.hmod` carries both at its root.
