#include "PBR.glsl"
struct TextureInfo {
    int index;
    int coord;
};

struct TransmissionInfo {
    TextureInfo transmission_texture;
    float transmission_factor;
    bool exist;
};

struct MetallicRoughnessInfo {
    float metallic_factor;
    float roughness_factor;
    TextureInfo metallic_roughness_texture;
};

struct VolumeInfo {
    vec3 attenuation_color;
    float thickness_factor;
    TextureInfo thickness_texture;
    float attenuation_distance;
    bool exists;
};

struct SpecularInfo {
    TextureInfo specular_texture;
    TextureInfo specular_color_texture;
    vec4 specular_color_factor;
    float specular_factor;
    bool exist;
    vec2 _padding;
};

struct MaterialRaw {
    uint alpha_mode;
	float alpha_cutoff;
	vec2 _padding;
    float _padding1;
    bool double_sided;

    TextureInfo baseColorTexture;
    vec4 baseColor;

    MetallicRoughnessInfo metallicRoughnessInfo;
    TextureInfo normal_texture;
    TextureInfo emissive_texture;
    vec4 emissive_factor;

    TextureInfo occlusion_texture;
    float ior;
    bool unlit;
    TransmissionInfo transmission_info;
    VolumeInfo volume_info;
    SpecularInfo specular_info;

};





