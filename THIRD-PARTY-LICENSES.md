# Third-Party Licenses

This document lists the licenses of third-party dependencies used in fuckupRSS.

## Project License

fuckupRSS is licensed under the **MIT License**. See [LICENSE](LICENSE) for details.

## Font Awesome Free 6.7.2

Included in `static/fontawesome/`.

| Component | License |
|-----------|---------|
| Icons | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |
| Fonts | [SIL OFL 1.1](https://scripts.sil.org/OFL) |
| Code | [MIT](https://opensource.org/licenses/MIT) |

Source: https://fontawesome.com/license/free

## Rust Dependencies (Backend)

533 crates total. The vast majority are licensed under **MIT**, **Apache-2.0**, or dual-licensed **MIT OR Apache-2.0**.

### Non-Permissive Licenses

| Crate | License | Compatibility Notes |
|-------|---------|---------------------|
| `cssparser`, `cssparser-macros` | MPL-2.0 | Compatible — MPL-2.0 is file-level copyleft, does not affect the overall project license. |
| `dtoa-short` | MPL-2.0 | Compatible — same as above. |
| `selectors` | MPL-2.0 | Compatible — same as above. |
| `option-ext` | MPL-2.0 | Compatible — same as above. |
| `r-efi` | MIT OR Apache-2.0 OR LGPL-2.1+ | Compatible — MIT/Apache-2.0 option available. |

No LGPL dependencies. The RAKE algorithm for keyword extraction uses a local implementation.

## npm Dependencies (Frontend)

63 production dependencies. License distribution:

| License | Count |
|---------|-------|
| MIT | 42 |
| ISC | 11 |
| MIT OR Apache-2.0 | 3 |
| Apache-2.0 | 2 |
| BSD-3-Clause | 1 |
| 0BSD | 1 |
| MPL-2.0 OR Apache-2.0 | 1 |

All npm production dependencies use permissive licenses compatible with MIT.

## Full Dependency Lists

To generate complete dependency lists:

```bash
# Rust dependencies
cargo metadata --manifest-path src-tauri/Cargo.toml --format-version 1 | \
  python3 -c "import sys,json; [print(f'{p[\"name\"]} {p.get(\"license\",\"?\")}') for p in sorted(json.load(sys.stdin)['packages'], key=lambda x: x['name'])]"

# npm dependencies
npx license-checker --production --csv
```
