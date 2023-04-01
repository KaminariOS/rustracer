#version 460
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_ray_tracing : require
//#include "Material.glsl"
#include "lib/Random.glsl"
#include "lib/RayTracingCommons.glsl"
#include "lib/Material.glsl"
#include "lib/PBR.glsl"

struct Vertex {
	vec3 pos;
	vec3 normal;
	vec4 tangent;
	vec3 color;
	vec2 uvs;
	uint material_index;
};

layout(binding = VERTEX_BIND, set = 0) readonly buffer Vertices { Vertex v[]; } vertices;
layout(binding = INDEX_BIND, set = 0) readonly buffer Indices { uint i[]; } indices;
layout(binding = GEO_BIND, set = 0) readonly buffer PrimInfos { PrimInfo p[]; } primInfos;
layout(binding = MAT_BIND, set = 0) readonly buffer Materials { MaterialRaw m[]; } materials;
layout(binding = TEXTURE_BIND) uniform sampler2D[] textures;

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

vec4 Mix(vec4 a, vec4 b, vec4 c, vec3 barycentrics)
{
	return a * barycentrics.x + b * barycentrics.y + c * barycentrics.z;
}

vec3 normal_transform(vec3 normal) {
	return normalize(vec3(normal * gl_WorldToObjectEXT));
}

void main()
{
	PrimInfo primInfo = primInfos.p[gl_InstanceCustomIndexEXT];
	MaterialRaw mat = materials.m[primInfo.material_id];

	Ray.instance_id = gl_InstanceID;
	Ray.bary = HitAttributes;
	// Fetch vertices
	uint vertexOffset = primInfo.v_offset;
	uint indexOffset = primInfo.i_offset + (3 * gl_PrimitiveID);

	uint i0 = vertexOffset + indices.i[indexOffset];
	uint i1 = vertexOffset + indices.i[indexOffset + 1];
	uint i2 = vertexOffset + indices.i[indexOffset + 2];

	Vertex v0 = vertices.v[i0];
	Vertex v1 = vertices.v[i1];
	Vertex v2 = vertices.v[i2];

	// Compute the ray hit point properties.
	const vec3 barycentricCoords = vec3(1.0 - HitAttributes.x - HitAttributes.y, HitAttributes.x, HitAttributes.y);
	const vec2 uvs = Mix(v0.uvs, v1.uvs, v2.uvs, barycentricCoords);

	vec3 normal = Mix(v0.normal, v1.normal, v2.normal, barycentricCoords);
	normal = normal_transform(normal);
//	if (mat.normal_texture.index > -1) {
//		vec3 normal_t = normalize(texture(textures[mat.normal_texture.index], uvs).xyz * 2. - 1.);
//
//		vec3 t = Mix(v0.tangent, v1.tangent, v2.tangent, barycentricCoords).xyz;
//
//		vec3 b = normal_transform(cross(normal, t));
//		t = normal_transform(t);
//		normal = normal_transform(normal);
//
//		mat3 tbn = mat3(t, b, normal);
//		normal = normalize(tbn * normal_t);
//	} else {
//		normal = normal_transform(normal);
//	}

	// Interpolate Color
	vec3 vertexColor = Mix(v0.color, v1.color, v2.color, barycentricCoords);
	vec3 baseColor = mat.baseColor.xyz;
	vec3 color = vertexColor * baseColor;

	if (mat.baseColorTexture.index > -1) {
		color = color * texture(textures[mat.baseColorTexture.index], uvs).rgb;
	}

	const vec3 pos = Mix(v0.pos, v1.pos, v2.pos, barycentricCoords);
	vec3 origin = vec3(gl_ObjectToWorldEXT * vec4(pos, 1.0)) ;
	origin = offset_ray(origin, normal);

	vec3 emittance = mat.emissive_factor.rgb;
	float metallic = mat.metallicFactor;
	float roughness = mat.roughnessFactor;
	float ior = mat.ior;

	Ray.hitPoint = origin;
	Ray.t = gl_HitTEXT;
	uint seed = Ray.RandomSeed;


	if (ior > 1.) {
		const float dot = dot(gl_WorldRayDirectionEXT, normal);
		const bool frontFace = dot < 0.;
		const vec3 outwardNormal = frontFace ? normal : -normal;
		const float refraction_ratio = frontFace ? 1 / ior: ior;
		const float cos_theta = abs(dot);
		const vec3 refracted = refract(gl_WorldRayDirectionEXT, outwardNormal, refraction_ratio);
		const float reflectProb = refracted != vec3(0) ? Schlick(cos_theta, refraction_ratio) : 1;

		Ray.hitValue =  color;
		Ray.needScatter = true;

		 if (RandomFloat(seed) < reflectProb) {
			 Ray.scatterDirection = reflect(gl_WorldRayDirectionEXT, normal);
		 } else {
			 Ray.scatterDirection = refracted;
		 }
	}
	else if (length(emittance) < 0.01 && roughness > 0.) {
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
		Ray.scatterDirection = reflected + 0.08 * RandomInUnitSphere(seed);
	}
	else {
		Ray.hitValue = color * emittance * 5.0;
		Ray.needScatter = false;
	}

	Ray.RandomSeed = seed;
}
