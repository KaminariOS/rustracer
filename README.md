<div align="center">

# `Rustracer`

![Lines of code](https://tokei.rs/b1/github/KaminariOS/rustracer)
![GitHub repo size](https://img.shields.io/github/repo-size/KaminariOS/rustracer)
[![dependency status](https://deps.rs/repo/github/KaminariOS/rustracer/status.svg)](https://deps.rs/repo/github/KaminariOS/rustracer)
![GitHub](https://img.shields.io/github/license/KaminariOS/rustracer)

</div>

A PBR [glTF 2.0](https://www.khronos.org/gltf) renderer based on Vulkan ray-tracing, written in Rust.

## Naming
This project and I are not affiliated with the [Rust Foundation](https://foundation.rust-lang.org). I name it `rus-tracer` only because I love [Rust](https://rust.facepunch.com) and ray tracing.

## Credits

This project is based on Adrien Ben's [vulkan-examples-rs](https://github.com/adrien-ben/vulkan-examples-rs). 

Sample accumulation implementation, heatmap and Lucy obj model are from project [Ray Tracing In Vulkan](https://github.com/GPSnoopy/RayTracingInVulkan).

I stole the PBR shaders from the [referencePT](https://github.com/boksajak/referencePT) project and made some changes.

## Demos
[rustracer-0.2.0-Demo videos](https://www.youtube.com/watch?v=HYZCgwP7rmA&list=LLeDjbBFLvS94sHm_j6i3CHw).

[rustracer-0.1.0-Demo videos](https://www.youtube.com/playlist?list=PLD1H28onwV_mFsPySwOtlBn9h5ybzepir).


![Lucy in Cornell](images/lucy.png)

## Features
* [x] Loading arbitrary glTF 2.0 models
  * [x] Full node hierarchy
    * [x] Mesh
      * [x] Geometry normal generation
      * [x] Two sets of texture coords
      * [x] Mikktspace tangent generation
      * [x] Normal mapping
    * [ ] Camera
  * [x] Alpha blending and testing
  * [x] Full PBR material support
    * [x] Metallic-Roughness workflow
    * [x] Specular-Glossiness workflow
  * [x] Animations
    * [x] Articulated (translate, rotate, scale)
    * [x] Skinning(using compute shader)
    * [ ] Morph targets
  * [x] Extensions
      * [x] "KHR_materials_ior",
      * [x] "KHR_materials_pbrSpecularGlossiness",
      * [x] "KHR_materials_transmission",
        * [ ] importance sampling and BTDF 
      * [x] "KHR_materials_variants",
        * [ ] GUI support
      * [x] "KHR_materials_volume",
      * [ ] "KHR_materials_specular",
      * [ ] "KHR_texture_transform",
      * [x] "KHR_materials_unlit",
      * [x] "KHR_lights_punctual",
* [x] Optimizations
  * [x] Rayon-accelerated texture loading
  * [x] Async model loading
  * [ ] Async acceleration structure building

* [ ] Realtime ray tracing 
  * [ ] Rasterization mode
  * [ ] G-buffer
  * [ ] Hybrid mode
  * [ ] SVGF denoiser
  * [ ] Path regularization
  * [ ] Better multi-light sampling like ReSTIR
  * [ ] Blue noise and Halton sequence
  
* [x] Extras
  * [x] Open file by drag-and-drop
  * [x] Skybox
  * [ ] Skydome(hdr)
  * [ ] Loading multiple glTF scene dynamically
  * [ ] Rigid-body simulation
    
## Building
### Prerequisites
- Linux and a graphics card that supports KHR ray tracing
  - Currently, I hard-coded all test model paths in an enum(in [`gui_state.rs`](crates/examples/gltf_viewer/src/gui_state.rs)) and load models in the search paths(see [`resource manager`](crates/libs/resource_manager)). 
  - You can open arbitrary glTF file by `cargo run -- -f <filename>` or drag-and-drop
- Windows not supported. Need some minor cfg tweaks to work on Windows. Open to pull requests.

### Build command
Thanks to the superior Rust package manager `Cargo`, building and running can be as brainless as a one-liner: `cargo run`. 

However, some external C libraries like Vulkan SDK may be missing on your system(those libraries are necessary for basically any Vulkan or non-trivial graphics programming project, regardless of whatever programming language used). 


- To install those libraries automatically,
  - Install [Nix](https://nixos.org/download.html) package manager(Linux only) and [direnv](https://direnv.net). 
  - `cd` into the project directory and `direnv allow`.
- To install external libraries manually
  - `cargo run` to find out what is missing and install it
  - Look into the list in [flake.nix](flake.nix).
  
## Assets
Pointers to glTF models: 
- [My collection](./assets/models/licenses)
- [glTF sample models](https://github.com/KhronosGroup/glTF-Sample-Models).
- [A ton of glTF models](https://sketchfab.com/search?features=downloadable&type=models)
- [Open Research Content Archive](https://developer.nvidia.com/orca) can be converted to glTF with Blender.
- [Poly heaven](https://polyhaven.com)

## References
- [boksajak/referencePT](https://github.com/boksajak/referencePT)
- [NVIDIA Vulkan Ray Tracing Tutorial](https://nvpro-samples.github.io/vk_raytracing_tutorial_KHR/)
- [adrien-ben/gltf-viewer-rs](https://github.com/adrien-ben/gltf-viewer-rs)
- [adrien-ben/vulkan-examples-rs](https://github.com/adrien-ben/vulkan-examples-rs)
- [GPSnoopy/RayTracingInVulkan](https://github.com/GPSnoopy/RayTracingInVulkan)
- [Ray Tracing Gems II](https://www.realtimerendering.com/raytracinggems/rtg2/index.html)
