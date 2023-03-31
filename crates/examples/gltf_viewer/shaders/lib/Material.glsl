struct TextureInfo {
    int index;
    int coord;
};

struct MaterialRaw {
    uint alpha_mode;
    bool double_sided;

    TextureInfo baseColorTexture;
    vec4 baseColor;

    float metallicFactor;
    float roughnessFactor;
    TextureInfo metallic_roughness_texture;
    TextureInfo normal_texture;
    TextureInfo emissive_texture;
    vec4 emissive_factor;

    TextureInfo occlusion_texture;
    float ior;
    uint padding;
};
