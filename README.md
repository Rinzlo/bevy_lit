![bevy_lit demo](https://github.com/malbernaz/bevy_lit/raw/main/static/demo.png)

# `bevy_lit`

`bevy_lit` is a simple and easy-to-use 2D lighting library for Bevy, designed to work seamlessly with a single camera setup. The library provides basic lighting functionalities through the types: `Lighting2dSettings`, `AmbientLight2d`, `LightOccluder2d`, and `PointLight2d`.

## Features

- **Lighting2dSettings**: Controls lighting parameters such as shadow softness
- **AmbientLight2d**: Provides a general light source that illuminates the entire scene uniformly.
- **PointLight2d**: Emits light from a specific point, simulating light sources like lamps or torches.
- **LightOccluder2d**: Creates shadows and blocks light from `PointLight2d`.
- Web support both for **WebGPU**

## Getting Started

### Installation

Install it using the CLI:

```sh
cargo add bevy_lit
```

Or add `bevy_lit` to your `Cargo.lock`:

```toml
[dependencies]
bevy_lit = "*"
```

### Usage

Below is a basic example demonstrating how to set up and use `bevy_lit` in your Bevy project:

```rust
use bevy::prelude::*;
use bevy_lit::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Lighting2dPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.spawn((
        Camera2d,
        Lighting2dSettings::default(),
    ));

    commands.spawn(PointLight2d {
        color: Color::WHITE,
        intensity: 3.0,
        radius: 200.0,
        falloff: 2.0,
        ..default(),
    });

    commands.spawn((
        Mesh2d(meshes.add(Circle::new(50.0))),
        LightOccluder2d::default()),
        Transform::from_xyz(0.0, 200.0, 0.0)
    ));
}
```

## Implementation

`bevy_lit` uses signed distance fields (SDFs) to compute the occluders' distances. To soften the shadows, a blur is applied. This approach is not ideal and might have limitations in terms of performance and visual accuracy, but it provides a starting point for basic 2D lighting effects.

## Compatibility

| bevy   | bevy_lit   |
| ------ | ---------- |
| `0.15` | `0.4..0.5` |
| `0.14` | `0.3`      |

## Acknowledgement

This library took heavy inspiration from the work of other developers. I learned a lot about lighting and Bevy development by reading the source code of the following crates:

- [`bevy_light_2d`](https://github.com/jgayfer/bevy_light_2d)
- [`bevy-magic-light-2d`](https://github.com/zaycev/bevy-magic-light-2d)

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

`bevy_lit` is licensed under the MIT License. See [LICENSE](LICENSE) for more details.
