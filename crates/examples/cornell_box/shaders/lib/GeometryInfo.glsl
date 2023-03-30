struct TextureInfo {
    int index;
    int coord;
};

struct GeometryInfo {
	mat4 transform;
	vec4 baseColor;
	vec4 emissive_factor;

	TextureInfo baseColorTexture;
	TextureInfo normal_texture;
    TextureInfo metallic_roughness_texture;

	float metallicFactor;
	float roughnessFactor;
	float ior;
	float _padding;
	uint vertexOffset;
	uint indexOffset;
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