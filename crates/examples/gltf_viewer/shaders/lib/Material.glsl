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


struct SpecularGlossiness {
    vec4 diffuse_factor;
    vec4 specular_glossiness_factor;
    TextureInfo diffuse_texture;
    TextureInfo specular_glossiness_texture;
};

const uint METALLIC_WORKFLOW = 0;
const uint SPECULAR_GLOSS_WORKFLOW = 1;

// https://developer.nvidia.com/blog/best-practices-for-using-nvidia-rtx-ray-tracing-updated/
// Use a separate hit shader for each material model(for example: metal?). Reducing code and data divergence within hit shaders is helpful, especially with incoherent rays. In particular, avoid übershaders that manually switch between material models. Implementing each required material model in a separate hit shader gives the system the best possibilities to manage divergent hit shading.
//
//When the material model allows a unified shader without much divergence, you can consider using a common hit shader for geometries with various materials.
struct MaterialRaw {
    uint alpha_mode;
	float alpha_cutoff;
    bool double_sided;
    uint workflow;
	vec2 _padding;

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
    SpecularGlossiness sg;
};

const float c_MinRoughness = 0.04;
// Gets metallic factor from specular glossiness workflow inputs
// https://github.com/SaschaWillems/Vulkan-glTF-PBR/blob/master/data/shaders/pbr_khr.frag
float convertMetallic(vec3 diffuse, vec3 specular, float maxSpecular) {
    float perceivedDiffuse = sqrt(0.299 * diffuse.r * diffuse.r + 0.587 * diffuse.g * diffuse.g + 0.114 * diffuse.b * diffuse.b);
    float perceivedSpecular = sqrt(0.299 * specular.r * specular.r + 0.587 * specular.g * specular.g + 0.114 * specular.b * specular.b);
    if (perceivedSpecular < c_MinRoughness) {
        return 0.0;
    }
    float a = c_MinRoughness;
    float b = perceivedDiffuse * (1.0 - maxSpecular) / (1.0 - c_MinRoughness) + perceivedSpecular - 2.0 * c_MinRoughness;
    float c = c_MinRoughness - perceivedSpecular;
    float D = max(b * b - 4.0 * a * c, 0.0);
    return clamp((-b + sqrt(D)) / (2.0 * a), 0.0, 1.0);
}




