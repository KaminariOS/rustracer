#version 460
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_ray_tracing : require
#include "lib/Random.glsl"
#include "lib/RayTracingCommons.glsl"
#include "lib/Material.glsl"
#include "lib/PBR.glsl"
#include "lib/UniformBufferObject.glsl"

layout(binding = VERTEX_BIND, set = 0) readonly buffer Vertices { Vertex v[]; } vertices;
layout(binding = INDEX_BIND, set = 0) readonly buffer Indices { uint i[]; } indices;
layout(binding = GEO_BIND, set = 0) readonly buffer PrimInfos { PrimInfo p[]; } primInfos;
layout(binding = MAT_BIND, set = 0) readonly buffer Materials { MaterialRaw m[]; } materials;
layout(binding = TEXTURE_BIND) uniform sampler2D[] textures;
layout(binding = PLIGHT_BIND) readonly buffer Lights { Light[] lights; };
layout(binding = UNIFORM_BIND, set = 0) readonly uniform UniformBufferObjectStruct { UniformBufferObject Camera; };

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
	const PrimInfo primInfo = primInfos.p[gl_InstanceCustomIndexEXT];
	const MaterialRaw mat = materials.m[primInfo.material_id];

//	Ray.instance_id = gl_InstanceID;
	Ray.t = gl_HitTEXT;
	// Fetch vertices
	const uint vertexOffset = primInfo.v_offset;
	const uint indexOffset = primInfo.i_offset + (3 * gl_PrimitiveID);

	const uint i0 = vertexOffset + indices.i[indexOffset];
	const uint i1 = vertexOffset + indices.i[indexOffset + 1];
	const uint i2 = vertexOffset + indices.i[indexOffset + 2];

	const Vertex v0 = vertices.v[i0];
	const Vertex v1 = vertices.v[i1];
	const Vertex v2 = vertices.v[i2];

	// Compute the ray hit point properties.
	const vec3 barycentricCoords = vec3(1.0 - HitAttributes.x - HitAttributes.y, HitAttributes.x, HitAttributes.y);
	const vec2 uvs = Mix(v0.uvs, v1.uvs, v2.uvs, barycentricCoords);
	const vec3 pos = Mix(v0.pos, v1.pos, v2.pos, barycentricCoords);
	vec3 origin = vec3(gl_ObjectToWorldEXT * vec4(pos, 1.0)) ;

	// Interpolate Color
	const vec3 vertexColor = Mix(v0.color, v1.color, v2.color, barycentricCoords);
	const vec3 baseColor = mat.baseColor.xyz;
	vec3 color = vertexColor * baseColor;
	if (mat.baseColorTexture.index >= 0) {
		color = color * texture(textures[mat.baseColorTexture.index], uvs).rgb;
	}

	Ray.needScatter = false;
	Ray.hitPoint = pos;
	switch (Camera.mapping) {
		case ALBEDO:
			Ray.hitValue = color;
			return;
		case TRIANGLE:
			Ray.hitValue = bary_to_color(HitAttributes);
			return;
		case INSTANCE:
			Ray.hitValue = hashAndColor(gl_InstanceID);
			return;
	}

	vec3 geo_normal = Mix(v0.normal, v1.normal, v2.normal, barycentricCoords);
	vec3 normal = geo_normal;
	if (mat.normal_texture.index >= 0) {
		vec3 normal_t = normalize(texture(textures[mat.normal_texture.index], uvs).xyz * 2. - 1.);

		vec3 t = Mix(v0.tangent, v1.tangent, v2.tangent, barycentricCoords).xyz;

		vec3 b = normal_transform(cross(normal, t));
		t = normal_transform(t);
		normal = normal_transform(normal);

		mat3 tbn = mat3(t, b, normal);
		// Shading normal
		normal = normalize(tbn * normal_t);
	} else {
		normal = normal_transform(normal);
	}

	origin = offset_ray(origin, normal);
	const float cos = dot(gl_WorldRayDirectionEXT, normal);
	const bool frontFace = cos < 0.;
	const vec3 outwardNormal = frontFace ? normal : -normal;
	geo_normal = frontFace? geo_normal: -geo_normal;

	vec3 emittance = mat.emissive_factor.rgb;
	if (mat.emissive_texture.index >= 0.) {
		emittance *= texture(textures[mat.emissive_texture.index], uvs).rgb;
	}

	const MetallicRoughnessInfo metallicRoughnessInfo = mat.metallicRoughnessInfo;
	float metallic = metallicRoughnessInfo.metallic_factor;
	float roughness = metallicRoughnessInfo.roughness_factor;
	const int mr_index = metallicRoughnessInfo.metallic_roughness_texture.index;

	if (mr_index >= 0.) {
		vec4 metallic_roughness = texture(textures[mr_index], uvs);
		roughness *= metallic_roughness.g;
		metallic *= metallic_roughness.b;
	}
	const float ior = mat.ior;

	Ray.hitPoint = origin;
	uint seed = Ray.RandomSeed;
	Ray.emittance = emittance * 1.0;
	Ray.needScatter = false;
	Ray.hitValue = vec3(1.);
//	uint brdfType;

//	if (metallic == 1.0 && roughness == 1.0) {
//		brdfType = SPECULAR_TYPE;
//	} else {
//		BRDF brdfProbability = getBrdfProbability(color, metallic, -gl_WorldRayDirectionEXT, outwardNormal);
//		if (RandomFloat(seed) < brdfProbability.specular) {
//			brdfType = SPECULAR_TYPE;
//			color /= brdfProbability.specular;
//		} else {
//			brdfType = DIFFUSE_TYPE;
//			color /= brdfProbability.diffuse;
//		}
//	}
//	MaterialBrdf matbrdf;
//	matbrdf.baseColor = color;
//	matbrdf.metallic = metallic;
//	matbrdf.roughness = roughness;
//	matbrdf.ior = ior;
//	vec3 brdfWeight;
//	vec2 u = vec2(RandomFloat(seed), RandomFloat(seed));
//	vec3 direction;
//	Ray.needScatter = evalIndirectCombinedBRDF(u, outwardNormal, geo_normal, -gl_WorldRayDirectionEXT, matbrdf, brdfType, direction, brdfWeight);
//	color *= brdfWeight;
//	Ray.hitPoint = origin;
//	Ray.scatterDirection = direction;
	if (ior > 1.) {

		const float refraction_ratio = frontFace ? 1 / ior: ior;
		const float cos_theta = abs(cos);
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
	}
	else if (metallic > 0.) {
		const vec3 reflected = reflect(gl_WorldRayDirectionEXT, normal);
		const bool isScattered = dot(reflected, normal) > 0;
		Ray.needScatter = isScattered;
		Ray.hitValue = isScattered? color: vec3(0.);
		Ray.scatterDirection = reflected + 0.08 * RandomInUnitSphere(seed);
	}

	Ray.RandomSeed = seed;
}
