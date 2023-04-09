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
layout(binding = UNIFORM_BIND, set = 0) readonly uniform UniformBufferObjectStruct { UniformBufferObject ubo; };

//#include "Scatter.glsl"
//#include "Vertex.glsl"

hitAttributeEXT vec2 HitAttributes;
rayPayloadInEXT RayPayload Ray;


vec3 normal_transform(vec3 normal) {
	return normalize(vec3(gl_ObjectToWorldEXT * vec4(normal, 0.)));
}

// Samples a random light from the pool of all lights using simplest uniform distirbution
bool sampleLightUniform(inout RngStateType rngState, vec3 hitPosition, vec3 surfaceNormal, out Light light, out float lightSampleWeight) {
	uint light_num = lights.length();
	if (light_num == 0) {
		return false;
	}

	uint randomLightIndex = min(light_num - 1, uint(rand(rngState) * light_num));
	light = lights[randomLightIndex];

	// PDF of uniform distribution is (1/light count). Reciprocal of that PDF (simply a light count) is a weight of this sample
	lightSampleWeight = float(light_num);

	return true;
}

bool sampleLightRIS(inout RngStateType rngState, vec3 hitPosition, vec3 surfaceNormal, out Light selectedSample, out float lightSampleWeight) {
	uint light_num = lights.length();
	if (light_num == 0) {
		return false;
	}
	float totalWeights = 0.0f;
	float samplePdfG = 0.0f;
	uint candidates_num = min(light_num, RIS_CANDIDATES_LIGHTS);
	for (int i = 0; i < candidates_num; i++) {

		float candidateWeight;
		Light candidate;
		if (sampleLightUniform(rngState, hitPosition, surfaceNormal, candidate, candidateWeight)) {

			vec3	lightVector = candidate.transform.xyz - hitPosition;
			float lightDistance = length(lightVector);;

			// Ignore backfacing light
			vec3 L = normalize(lightVector);
			if (dot(surfaceNormal, L) < 0.00001f) continue;

			#if SHADOW_RAY_IN_RIS
			// Casting a shadow ray for all candidates here is expensive, but can significantly decrease noise
			if (!castShadowRay(hitPosition, surfaceNormal, L, lightDistance)) continue;
			#endif

			float candidatePdfG = luminance(getLightIntensityAtPoint(candidate, length(lightVector)));
			const float candidateRISWeight = candidatePdfG * candidateWeight;

			totalWeights += candidateRISWeight;
			if (rand(rngState) < (candidateRISWeight / totalWeights)) {
				selectedSample = candidate;
				samplePdfG = candidatePdfG;
			}
		}
	}

	if (totalWeights == 0.0f) {
		return false;
	} else {
		lightSampleWeight = (totalWeights / float(RIS_CANDIDATES_LIGHTS)) / samplePdfG;
		return true;
	}
}

// https://github.com/KhronosGroup/glTF/issues/1252
void getNormal(inout vec3 normal_mix, vec4 tangent_mix, vec3 tex_normal) {
//	if (
//	length(tangent0) == 0 || tangent0.w == 0 ||
//	length(tangent1) == 0 || tangent1.w == 0 ||
//	length(tangent2) == 0 || tangent2.w == 0
//	) {
//		return;
//	}
//	vec3 tangent = Mix(
//		cross(n0, tangent0.xyz) * tangent0.w,
//		cross(n1, tangent1.xyz) * tangent1.w,
//		cross(n2, tangent2.xyz) * tangent2.w,
//		bary
//	);
	vec3 tangent = normalize((tangent_mix.xyz - dot(tangent_mix.xyz, normal_mix)*normal_mix) * tangent_mix.w);
//	vec3 tangent = normalize(tangent_mix.xyz * tangent_mix.w);
//	if (length(tex_normal) == 0) {
//		return;
//	}
//	if (tex_normal.z <= 0) {
//		tex_normal.z = sqrt(1 - tex_normal.x * tex_normal.x - tex_normal.y * tex_normal.y);
//	}
	vec3 b = cross(normal_mix, tangent);
	mat3 tbn = mat3(tangent, normalize(b), normal_mix);
	normal_mix = tbn * tex_normal;
}

void zero_raypayload() {
	Ray.needScatter = false;
	Ray.emittance = vec3(0);
}

void main()
{
	const PrimInfo primInfo = primInfos.p[gl_InstanceCustomIndexEXT];
	const MaterialRaw mat = materials.m[primInfo.material_id];

//	Ray.instance_id = gl_InstanceID;
	vec3 last_hit = Ray.hitPoint;
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
	Vertex mix_vertex;
	vec3 geo_normal = normal_transform(getMixVertexAndGeoNormal(v0, v1, v2, vec2(HitAttributes), mix_vertex));
	vec3 pos = mix_vertex.pos;
	vec3 origin = vec3(gl_ObjectToWorldEXT * vec4(pos, 1.0)) ;

	vec4 uv0And1= mix_vertex.uv0And1;

	// Interpolate Color
	const vec4 baseColor = mat.baseColor;
	vec4 color4 = mix_vertex.color * baseColor;
	TextureInfo baseColorTexture = mat.baseColorTexture;
	if (baseColorTexture.index >= 0) {
		color4 *= texture(textures[baseColorTexture.index],
		getUV(uv0And1, baseColorTexture.coord)
		);
	}
	vec3 color = color4.rgb;
	Ray.needScatter = false;
	Ray.hitPoint = pos;
	uint mapping = ubo.mapping;
	if (mat.unlit) {
		mapping = ALBEDO;
	}
	switch (mapping) {
		case ALBEDO:
			Ray.emittance = color;
			return;
		case TRIANGLE:
			Ray.emittance = bary_to_color(HitAttributes);
			return;
		case INSTANCE:
			Ray.emittance = hashAndColor(gl_InstanceID);
			return;
	}

	vec3 normal = mix_vertex.normal;
	TextureInfo normal_texture = mat.normal_texture;
	if (normal_texture.index >= 0
//	&& ubo.debug == 0
	) {
		vec3 normal_t = normalize(texture(textures[normal_texture.index], getUV(uv0And1, normal_texture.coord)).xyz * 2. - 1.);
		getNormal(normal, mix_vertex.tangent, normal_t);
	}
	normal = normal_transform(normal);

//	normal = geo_normal;
	const vec3 V = -gl_WorldRayDirectionEXT;
	const float cos = dot(V, geo_normal);
	const bool frontFace = cos >= 0.;
	geo_normal = frontFace? geo_normal: -geo_normal;
	const vec3 outwardNormal = dot(geo_normal, normal) < 0.? -normal: normal;

	origin = offset_ray(origin, geo_normal);
	vec3 emittance = mat.emissive_factor.rgb;
	if (mat.emissive_texture.index >= 0.) {
		emittance *= texture(textures[mat.emissive_texture.index],
		getUV(uv0And1, mat.emissive_texture.coord)
		).rgb;
	}

	const MetallicRoughnessInfo metallicRoughnessInfo = mat.metallicRoughnessInfo;
	float metallic = metallicRoughnessInfo.metallic_factor;
	float roughness = metallicRoughnessInfo.roughness_factor;
	const int mr_index = metallicRoughnessInfo.metallic_roughness_texture.index;
	const TextureInfo mr_tex = metallicRoughnessInfo.metallic_roughness_texture;
	if (mr_index >= 0.) {
		vec4 metallic_roughness = texture(textures[mr_index], getUV(uv0And1, mr_tex.coord));
		roughness *= metallic_roughness.g;
		metallic *= metallic_roughness.b;
	}

	const TransmissionInfo trans_info = mat.transmission_info;
	const TextureInfo trans_tex = trans_info.transmission_texture;
	float transmission_factor = 0.;
	if (trans_info.exist) {
		transmission_factor = trans_info.transmission_factor;
		if (trans_tex.index >= 0) {
			transmission_factor *= texture(textures[trans_tex.index], getUV(uv0And1, trans_tex.coord)).r;
		}
	}

	SpecularInfo spec_info = mat.specular_info;
	float spec_factor = spec_info.specular_factor;
	vec3 spec_color_factor = spec_info.specular_color_factor.rgb;
	TextureInfo specular_texture = spec_info.specular_texture;
	if (specular_texture.index >= 0) {
		spec_factor *= texture(textures[specular_texture.index], getUV(uv0And1, specular_texture.coord)).a;
	}
	TextureInfo specular_color_texture = spec_info.specular_color_texture;
	if (specular_color_texture.index >= 0) {
		spec_color_factor *= texture(textures[specular_color_texture.index],
		getUV(uv0And1, specular_color_texture.coord)
		).rgb;
	}
	const float ior = mat.ior;

	VolumeInfo volume_info = mat.volume_info;

	Ray.hitPoint = origin;
	uint seed = Ray.RandomSeed;
	RngStateType rngState = Ray.rngState;

	Ray.emittance = emittance * 1.0;
	Ray.needScatter = false;
	Ray.hitValue = vec3(1.);
	uint brdfType;

	vec3 throughput = color;

	MaterialBrdf matbrdf;
	matbrdf.baseColor = color;
	matbrdf.metallic = metallic;
	matbrdf.roughness = roughness;
	matbrdf.ior = ior;
	matbrdf.transmission = transmission_factor;
	matbrdf.specular_factor = spec_factor;
	matbrdf.specular_color_factor = spec_color_factor;
	matbrdf.use_spec = spec_info.exist;
	matbrdf.frontFace = frontFace;
	matBuild(matbrdf);
	matbrdf.attenuation_color = volume_info.attenuation_color;
	matbrdf.attenuation_distance = volume_info.attenuation_distance;
	float displacement = length(origin - last_hit);
	matbrdf.t_diff = displacement;

	if (metallic == 1.0 && roughness == 0.0) {
		brdfType = SPECULAR_TYPE;
	}
	else {
		BRDF brdfProbability = getBrdfProbability(matbrdf, -gl_WorldRayDirectionEXT, outwardNormal);
		float randfloat = rand(rngState);

		if (randfloat < brdfProbability.specular) {
			brdfType = SPECULAR_TYPE;
			throughput /= brdfProbability.specular;
			if (Ray.volume_dis >= 0) {
				// still in volume
				Ray.volume_dis += displacement;
			}
		} else if (randfloat >= brdfProbability.specular && randfloat <= brdfProbability.specular + brdfProbability.diffuse) {
			brdfType = DIFFUSE_TYPE;
			throughput /= brdfProbability.diffuse;
			if (Ray.volume_dis >= 0) {
				// still in volume
				Ray.volume_dis += displacement;
			}
		} else {
			brdfType = TRANSMISSION_TYPE;
			if (Ray.volume_dis >= 0) {
				// still in volume
				Ray.volume_dis += displacement;
			}
			else if (frontFace) {
				// enter volume
				Ray.volume_dis = 0;
			}
			 else {
				zero_raypayload();
				return;
			}
			throughput /= brdfProbability.transmission;
		}
	}

	vec3 brdfWeight;
	vec2 u = vec2(rand(rngState), rand(rngState));
	vec3 direction;
	Ray.needScatter = evalIndirectCombinedBRDF(u, outwardNormal, geo_normal,
	-gl_WorldRayDirectionEXT,
	matbrdf,
	brdfType,
	direction,
	brdfWeight,
	Ray.volume_dis
	);


	throughput *= brdfWeight;
	Ray.hitPoint = origin;
	Ray.scatterDirection = direction;
	Ray.hitValue = throughput;

////	Ray.emittance = vec3(hashAndColor(brdfType));
	//	Ray.needScatter = false;
//	if (dot(-gl_WorldRayDirectionEXT, geo_normal) > 0) {
//		Ray.emittance = vec3(1.);
//	} else {
//		Ray.emittance = vec3(0);
//	}
//	if (brdfType == SPECULAR_TYPE) {
//		Ray.emittance = vec3(brdfWeight);
//	}
//	Ray.hitValue = vec3(0.);

//	if (brdfType == DIFFUSE_TYPE) {
//
//	}
//	else {
//		Ray.emittance = vec3(0.);
//	}

//	if (matbrdf.transmission > 0.) {
//
//		const float refraction_ratio = frontFace ? 1 / ior: ior;
//		const float cos_theta = abs(cos);
//		const vec3 refracted = refract(gl_WorldRayDirectionEXT, outwardNormal, refraction_ratio);
//		const float reflectProb = refracted != vec3(0) ? Schlick(cos_theta, refraction_ratio) : 1;
//
//		Ray.hitValue =  color;
//		Ray.needScatter = true;
//
//		 if (RandomFloat(seed) < reflectProb) {
//			 Ray.scatterDirection = reflect(gl_WorldRayDirectionEXT, normal);
//		 } else {
//			 Ray.scatterDirection = refracted;
//		 }
//	}
//	else
//if (length(emittance) < 0.01 && roughness == 1.) {
//		const bool isScattered = dot(gl_WorldRayDirectionEXT, outwardNormal) < 0.00;
//		const vec3 scatter = vec3(normal + RandomInUnitSphere(seed));
//		Ray.needScatter = isScattered;
//		Ray.scatterDirection = scatter;
//		Ray.hitValue = isScattered? color: vec3(0.);
//	}
//	else if (metallic > 0.) {
//		const vec3 reflected = reflect(gl_WorldRayDirectionEXT, normal);
//		const bool isScattered = dot(reflected, normal) > 0;
//		Ray.needScatter = isScattered;
//		Ray.hitValue = isScattered? color: vec3(0.);
//		Ray.scatterDirection = reflected + 0.08 * RandomInUnitSphere(seed);
//	}

	Ray.RandomSeed = seed;
	Ray.rngState = rngState;
}
