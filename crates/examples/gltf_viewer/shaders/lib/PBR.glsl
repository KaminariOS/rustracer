#extension GL_EXT_nonuniform_qualifier : require

// BRDF types
#define DIFFUSE_TYPE 1
#define SPECULAR_TYPE 2
#define TRANSMISSION_TYPE 3
// Polynomial approximation by Christophe Schlick
float Schlick(const float cosine, const float refractionIndex)
{
	float r0 = (1 - refractionIndex) / (1 + refractionIndex);
	r0 *= r0;
	return r0 + (1 - r0) * pow(1 - cosine, 5);
}
