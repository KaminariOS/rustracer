#version 460
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_ray_tracing : require
//#include "Material.glsl"
#include "lib/Random.glsl"
#include "lib/RayTracingCommons.glsl"
#include "lib/PBR.glsl"

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


struct Vertex {
	vec3 pos;
	vec3 normal;
	vec3 color;
	vec2 uvs;
};

layout(binding = 3, set = 0) readonly buffer Vertices { Vertex v[]; } vertices;
layout(binding = 4, set = 0) readonly buffer Indices { uint i[]; } indices;
layout(binding = 5, set = 0) readonly buffer GeometryInfos { GeometryInfo g[]; } geometryInfos;
layout(binding = 6) uniform sampler2D[] textures;

//#include "Scatter.glsl"
//#include "Vertex.glsl"

hitAttributeEXT vec2 HitAttributes;
rayPayloadInEXT RayPayload Ray;

vec2 Mix(vec2 a, vec2 b, vec2 c, vec3 barycentrics)
{
	return a * barycentrics.x + b * barycentrics.y + c * barycentrics.z;
}

vec3 Mix(vec3 a, vec3 b, vec3 c, vec3 barycentrics) 
{
    return a * barycentrics.x + b * barycentrics.y + c * barycentrics.z;
}

void main()
{
	GeometryInfo geometryInfo = geometryInfos.g[gl_GeometryIndexEXT];

	// Fetch vertices
	uint vertexOffset = geometryInfo.vertexOffset;
	uint indexOffset = geometryInfo.indexOffset + (3 * gl_PrimitiveID);

	uint i0 = vertexOffset + indices.i[indexOffset];
	uint i1 = vertexOffset + indices.i[indexOffset + 1];
	uint i2 = vertexOffset + indices.i[indexOffset + 2];

	Vertex v0 = vertices.v[i0];
	Vertex v1 = vertices.v[i1];
	Vertex v2 = vertices.v[i2];

	// Compute the ray hit point properties.
	const vec3 barycentricCoords = vec3(1.0 - HitAttributes.x - HitAttributes.y, HitAttributes.x, HitAttributes.y);
	vec3 normal = normalize(Mix(v0.normal, v1.normal, v2.normal, barycentricCoords));
//	normal = normalize(vec3(normal * gl_WorldToObjectEXT));
	normal = normalize(geometryInfo.transform * vec4(normal, 0.0)).xyz;
	const vec2 uvs = Mix(v0.uvs, v1.uvs, v2.uvs, barycentricCoords);

	// Interpolate Color
	vec3 vertexColor = Mix(v0.color, v1.color, v2.color, barycentricCoords);
	vec3 baseColor = geometryInfo.baseColor.xyz;
	vec3 color = vertexColor * baseColor;

	if (geometryInfo.baseColorTextureIndex > -1) {
		color = color * texture(textures[geometryInfo.baseColorTextureIndex], uvs).rgb;
	}

	const vec3 pos = Mix(v0.pos, v1.pos, v2.pos, barycentricCoords);
	vec3 origin = vec3(geometryInfo.transform * vec4(pos, 1.0)) ;
//	origin = offset_ray(origin, normal);

	vec3 emittance = geometryInfo.emissive_factor.rgb;
	float metallic = geometryInfo.metallicFactor;
	float roughness = geometryInfo.roughnessFactor;
	float ior = geometryInfo.ior;

	Ray.hitPoint = origin;
	Ray.t = gl_HitTEXT;
	uint seed = Ray.RandomSeed;


	if (ior > 1.) {
		const float dot = dot(gl_WorldRayDirectionEXT, normal);
		const vec3 outwardNormal = dot > 0 ? -normal : normal;
		const float niOverNt = dot > 0 ? ior : 1 / ior;
		const float cosine = dot > 0 ? ior * dot : -dot;
		const vec3 refracted = refract(gl_WorldRayDirectionEXT, outwardNormal, niOverNt);
		const float reflectProb = refracted != vec3(0) ? Schlick(cosine, ior) : 1;

		Ray.hitValue =  color;
		Ray.needScatter = true;

		 if (RandomFloat(seed) < reflectProb) {
			 Ray.scatterDirection = reflect(gl_WorldRayDirectionEXT, normal);
		 } else {
			 Ray.scatterDirection = refracted;
		 }
	}
	else if (length(emittance) < 0.01 && roughness >= 1.) {

		const bool isScattered = dot(gl_WorldRayDirectionEXT, normal) < 0.;
		const vec3 scatter = vec3(normal + RandomInUnitSphere(seed));
		Ray.needScatter = isScattered;
		Ray.scatterDirection = scatter;
		Ray.hitValue = isScattered? color: vec3(0.);
	} else if (metallic > 0.) {

		const vec3 reflected = reflect(gl_WorldRayDirectionEXT, normal);
		const bool isScattered = dot(reflected, normal) > 0;
		Ray.needScatter = isScattered;
		Ray.hitValue = isScattered? color: vec3(0.);
		Ray.scatterDirection = reflected;
	}
	else {
		Ray.hitValue = color * emittance;
		Ray.needScatter = false;
	}

	Ray.RandomSeed = seed;
}
