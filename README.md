# Rustracer

A glTF 2.0 renderer based on Vulkan ray-tracing, written in Rust.

This project is based on Adrien Ben's [vulkan-examples-rs](https://github.com/adrien-ben/vulkan-examples-rs).

[Demo videos]()

I stole the PBR shaders from the [referencePT](https://github.com/boksajak/referencePT) project and made some changes.

## Features
* [x] Loading arbitrary glTF 2.0 models
  * [x] Full node hierarchy
    * [ ] Camera
    * [x] Mesh
      * [x] Geometry normal generation
      * [x] Two sets of texture coords
      * [x] Mikktspace tangent generation
      * [x] Normal mapping
      
  * [x] Alpha blending and testing
  * [x] Full PBR material support
    * [x] Metallic-Roughness workflow
    * [ ] Specular-Glossiness workflow
  * [x] Animations
    * [x] Articulated (translate, rotate, scale)
    * [ ] Skinning
    * [ ] Morph targets
  * [x] Extensions
      * [x] "KHR_materials_ior",
      * [ ] "KHR_materials_pbrSpecularGlossiness",
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
  * [ ] G-buffer Rasterization mode
  * [ ] Hybrid mode
  * [ ] SVGF denoiser
  * [ ] Path regularization
  * [ ] Better multi-light sampling like ReSTIR
  * [ ] Blue noise and Halton sequence
  
* [x] Extras
  * [x] Skybox
  * [ ] Skydome(hdr)
  * [ ] Drop file
  * [ ] Loading multiple glTF scene dynamically
  * [ ] Rigid-body simulation
    
## Building
### Prerequisites
- Linux and a graphics card that supports KHR ray tracing
- Windows not supported. Need some minor cfg tweaks to work on Windows. Open to pull requests.


Thanks to the superior Rust package manager `Cargo`, building and running is almost as simple as a one-liner: `cargo run`. 

However, some external C libraries like Vulkan SDK may be missing on your system. 
- To install those libraries automatically,
  - Install [Nix](https://nixos.org/download.html) package manager(Linux only) and [direnv](https://direnv.net). 
  - `cd` into the project directory and `direnv allow`.
- To install external libraries manually
  - `cargo run` to find out what is missing and install it
  - Look into the list in [flake.nix](flake.nix).
  
## Assets
A pointer to glTF models: 
- [glTF sample models](https://github.com/KhronosGroup/glTF-Sample-Models).
- [Open Research Content Archive](https://developer.nvidia.com/orca) can be converted to glTF with Blender.
- [Poly heaven](https://polyhaven.com)

## References
- [boksajak/referencePT](https://github.com/boksajak/referencePT)
- [NVIDIA Vulkan Ray Tracing Tutorial](https://nvpro-samples.github.io/vk_raytracing_tutorial_KHR/)
- [adrien-ben/gltf-viewer-rs](https://github.com/adrien-ben/gltf-viewer-rs)
- [adrien-ben/vulkan-examples-rs](https://github.com/adrien-ben/vulkan-examples-rs)
- [GPSnoopy/RayTracingInVulkan](https://github.com/GPSnoopy/RayTracingInVulkan)
