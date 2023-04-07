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
	return normalize(vec3(normal * gl_WorldToObjectEXT));
}

vec3 calculate_geo_normal(const vec3 p0, const vec3 p1, const vec3 p2) {
	vec3 v1 = p2 - p0;
	vec3 edge21 = p2 - p1;
	vec3 v0 = p1 - p0;
	return normal_transform(cross(v0, v1));
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

void getNormal(inout vec3 normal, Vertex v0, Vertex v1, Vertex v2, vec3 bary, vec3 tex_normal) {
	vec4 tangent0 = v0.tangent;
	vec4 tangent1 = v1.tangent;
	vec4 tangent2= v2.tangent;
	vec3 n0 = v0.normal;
	vec3 n1 = v1.normal;
	vec3 n2 = v2.normal;
	if (
	length(tangent0) == 0 || tangent0.w == 0 ||
	length(tangent1) == 0 || tangent1.w == 0 ||
	length(tangent2) == 0 || tangent2.w == 0
	) {
		return;
	}
	vec3 tangent = Mix(
		cross(n0, tangent0.xyz) * tangent0.w,
		cross(n1, tangent1.xyz) * tangent1.w,
		cross(n2, tangent2.xyz) * tangent2.w,
		bary
	);
	vec3 normal_mix = Mix(n0, n1, n2, bary);
	if (length(tex_normal) == 0) {
		return;
	}
	if (tex_normal.z <= 0) {
		tex_normal.z = sqrt(1 - tex_normal.x * tex_normal.x - tex_normal.y * tex_normal.y);
	}
	vec3 b = cross(normal_mix, tangent);
	mat3 tbn = mat3(normalize(tangent), normalize(b), normalize(normal_mix));
	normal = tbn * tex_normal;
}

void main()
{
	const PrimInfo primInfo = primInfos.p[gl_InstanceCustomIndexEXT];
	const MaterialRaw mat = materials.m[primInfo.material_id];

//	Ray.instance_id = gl_InstanceID;
	float last_t = Ray.t;
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
	const vec3 baseColor = mat.baseColor.rgb;
	vec3 color = vertexColor * baseColor;
	if (mat.baseColorTexture.index >= 0) {
		color = color * texture(textures[mat.baseColorTexture.index], uvs).rgb;
	}

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

	vec3 geo_normal = calculate_geo_normal(v0.pos, v1.pos, v2.pos);
	vec3 normal = Mix(v0.normal, v1.normal, v2.normal, barycentricCoords);
	if (mat.normal_texture.index >= 0 && ubo.debug == 0) {
		vec3 normal_t = normalize(texture(textures[mat.normal_texture.index], uvs).xyz * 2. - 1.);
		getNormal(normal, v0, v1, v2, barycentricCoords, normal_t);
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
		emittance *= texture(textures[mat.emissive_texture.index], uvs).rgb;
	}

	const MetallicRoughnessInfo metallicRoughnessInfo = mat.metallicRoughnessInfo;
	float metallic = metallicRoughnessInfo.metallic_factor;
	float roughness = metallicRoughnessInfo.roughness_factor;
	const int mr_index = metallicRoughnessInfo.metallic_roughness_texture.index;

	TransmissionInfo trans_info = mat.transmission_info;
	float transmission_factor = 0.;
	if (mat.transmission_info.exist) {
		transmission_factor = trans_info.transmission_factor;
		if (trans_info.transmission_texture.index >= 0) {
			transmission_factor *= texture(textures[trans_info.transmission_texture.index], uvs).r;
		}
	}
	if (mr_index >= 0.) {
		vec4 metallic_roughness = texture(textures[mr_index], uvs);
		roughness *= metallic_roughness.g;
		metallic *= metallic_roughness.b;
	}

	SpecularInfo spec_info = mat.specular_info;
	float spec_factor = spec_info.specular_factor;
	vec3 spec_color_factor = spec_info.specular_color_factor.rgb;
	if (spec_info.specular_texture.index >= 0) {
		spec_factor *= texture(textures[spec_info.specular_texture.index], uvs).a;
	}
	if (spec_info.specular_color_texture.index >= 0) {
		spec_color_factor *= texture(textures[spec_info.specular_color_texture.index], uvs).rgb;
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
	matbrdf.t_diff = gl_HitTEXT - last_t;

	if (metallic == 1.0 && roughness == 0.0) {
		brdfType = SPECULAR_TYPE;
	}
	else {
		BRDF brdfProbability = getBrdfProbability(matbrdf, -gl_WorldRayDirectionEXT, outwardNormal);
		float randfloat = rand(rngState);

		if (randfloat < brdfProbability.specular) {
			brdfType = SPECULAR_TYPE;
			throughput /= brdfProbability.specular;
		} else if (randfloat >= brdfProbability.specular && randfloat <= brdfProbability.specular + brdfProbability.diffuse) {
			brdfType = DIFFUSE_TYPE;
			throughput /= brdfProbability.diffuse;
		} else {
			brdfType = TRANSMISSION_TYPE;
			throughput /= brdfProbability.transmission;
		}
	}

	vec3 brdfWeight;
	vec2 u = vec2(rand(rngState), rand(rngState));
	vec3 direction;
	Ray.needScatter = evalIndirectCombinedBRDF(u, outwardNormal, geo_normal, -gl_WorldRayDirectionEXT, matbrdf, brdfType, direction, brdfWeight);


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
