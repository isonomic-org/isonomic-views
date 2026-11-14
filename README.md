# isonomic-views

Isonomic Views: a Rust engine that renders a small declarative document into native Isonomic UI
components, live, while the document is still being written. It is how artifacts work in
Isonomic Chat.

```xml
<View version="0.1" shared={{ seats: 25, price: 49 }}>
  <Col class="gap-3 p-4">
    <Heading>Seat cost calculator</Heading>
    <Slider bind={shared.seats} min={1} max={500} />
    <NumberInput bind={shared.price} prefix="$" />
    <Stat label="Annual cost" value={currency(shared.seats * shared.price * 12)} />
  </Col>
</View>
```

- Spec: [docs/spec.md](docs/spec.md). Status: [docs/status.md](docs/status.md).
  Resume point: [docs/PROGRESS.md](docs/PROGRESS.md).
- Chat runtime contract: [docs/artifact-contract.md](docs/artifact-contract.md).

## Commands

```sh
cargo test                       # headless crates (default members)
cargo test --workspace           # everything, including the renderer (needs a GPU or Mesa)
cargo run -p views-playground    # the live editor
cargo xtask screenshots          # render the corpus offscreen, light and dark
cargo xtask report               # parse / evaluate / build timings
cargo xtask guide                # regenerate the authoring guide and JSON Schema
cargo xtask deps                 # dependency allow-list for the headless crates
```

On Linux the renderer needs Vulkan; Mesa's software driver (`mesa-vulkan-drivers`) is enough
for offscreen screenshots.
