struct GeometryInfo {
	mat4 transform;
	vec4 baseColor;
	vec4 emissive_factor;
	int baseColorTextureIndex;
	float metallicFactor;
	float roughnessFactor;
	float ior;
	float _padding;
	float _padding2;
	uint vertexOffset;
	uint indexOffset;
};
