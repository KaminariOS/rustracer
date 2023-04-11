# Rustracer

This project is based on Adrien Ben's [vulkan-examples-rs](https://github.com/adrien-ben/vulkan-examples-rs).

I stole the PBR shaders from the [referencePT](https://github.com/boksajak/referencePT) project and made some changes.

## Features
* [x] Loading arbitrary glTF 2.0 models
  * [*] Full node hierarchy
    * [] Camera
    * [] Mesh
      * [x] Geometry normal generation
      * 
      * [x] Two tex coords
      * [x] Mikktspace tangent generation
    * 
  * [x] Full PBR material support
    * [x] Metallic-Roughness workflow
    * [] Specular-Glossiness workflow
  * [x] Animations
    * [x] Articulated (translate, rotate, scale)
    * [] Skinned
    * [] Morph targets
  * [x] Extensions
      * [x] "KHR_materials_ior",
      *[] "KHR_materials_pbrSpecularGlossiness",
      *[x] "KHR_materials_transmission",
        * [] importance sampling and BTDF 
      *[] "KHR_materials_variants",
        * [] GUI support
      *[x] "KHR_materials_volume",
      * [] "KHR_materials_specular",
      *[] "KHR_texture_transform",
      *[x] "KHR_materials_unlit",
      *[x] "KHR_lights_punctual",
  *[x] Performance
    * [x] Rayon-accelerated texture loading
    * [] Async acceleration structure building 
    
## Building
### Prerequisites
- Linux and a graphics card that supports KHR ray tracing
- Windows not supported. Need some minor cfg tweaks to work on Windows. Open to pull requests.


Thanks to the amazing Rust package manager Cargo, building and running is almost as simple as a one-liner: `cargo run`. 

However, some external C libraries like Vulkan SDK may be missing on you system. 
- To install those libraries automatically,
  - Install [Nix](https://nixos.org/download.html) package manager(Linux only) and [direnv](https://direnv.net). 
  - `cd` into the project directory and `direnv allow`.
- To install external libraries manually
  - `cargo run` to find out what is missing and install it
  - Look into the list in [flake.nix](flake.nix).
  
## Assets
A pointer to glTf models: [glTF sample models](https://github.com/KhronosGroup/glTF-Sample-Models).

## References
- [boksajak/referencePT](https://github.com/boksajak/referencePT)
- [NVIDIA Vulkan Ray Tracing Tutorial](https://nvpro-samples.github.io/vk_raytracing_tutorial_KHR/)
- [adrien-ben/gltf-viewer-rs](https://github.com/adrien-ben/gltf-viewer-rs)
- [adrien-ben/vulkan-examples-rs](https://github.com/adrien-ben/vulkan-examples-rs)
- [GPSnoopy/RayTracingInVulkan](https://github.com/GPSnoopy/RayTracingInVulkan)