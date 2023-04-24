#include "Random.glsl"
#include "PunctualLight.glsl"
#define STANDARD_RAY_INDEX 0
#define SHADOW_RAY_INDEX 1
#define SHADOW_RAY_IN_RIS 0

// ALLOW_UPDATE
// Consider writing a safe default value to unused payload fields.
// When some shader doesn’t use all fields in the payload, which are required by other shaders, it can be beneficial to still write a safe default value to the unused fields.
// This allows the compiler to discard the unused input value and use the payload register for other purposes before writing to it.
struct RayPayload
{
	vec3 hitValue;
	vec3 hitPoint;
	float t;
	vec3 scatterDirection;
	bool needScatter;
	uint RandomSeed;
	vec3 emittance;
	RngStateType rngState;
	float volume_dis;
//	uint instance_id;
//	vec2 bary;
};

struct ShadowRay {
	bool shadow;
	RngStateType rngState;
};


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

struct PrimInfo {
    uint v_offset;
    uint i_offset;
    uint material_id;
    uint _padding;
};

struct Vertex {
	vec3 pos;
	vec3 normal;
	vec4 tangent;
	vec4 color;
	vec4 weights;
	uvec4 joints;
	vec4 uv0And1;
	int skin_index;
};

vec2 getUV(vec4 uv0And1, uint index) {
	if (index == 0) {
		return uv0And1.xy;
	} else if (index == 1) {
		return uv0And1.zw;
	}
	return vec2(0.);
}


vec3 calculate_geo_normal(const vec3 p0, const vec3 p1, const vec3 p2) {
	vec3 v1 = p2 - p0;
	vec3 edge21 = p2 - p1;
	vec3 v0 = p1 - p0;
	return cross(v0, v1);
}

vec3 getMixVertexAndGeoNormal(const Vertex v0,
const Vertex v1,
const Vertex v2,
const vec2 attrs,
out Vertex mix_v) {
	const vec3 barycentricCoords = vec3(1.0 - attrs.x - attrs.y, attrs.x, attrs.y);

	mix_v.uv0And1 = Mix(v0.uv0And1, v1.uv0And1, v2.uv0And1, barycentricCoords);
	mix_v.pos = Mix(v0.pos, v1.pos, v2.pos, barycentricCoords);
	mix_v.color = Mix(v0.color, v1.color, v2.color, barycentricCoords);
	mix_v.normal = normalize(Mix(normalize(v0.normal), normalize(v1.normal),
	normalize(v2.normal), barycentricCoords));
	mix_v.tangent = normalize(Mix(v0.tangent, v1.tangent, v2.tangent, barycentricCoords));
	mix_v.skin_index = v0.skin_index;
	return calculate_geo_normal(v0.pos, v1.pos, v2.pos);
}

vec3 bary_to_color(vec2 bary) {
    return vec3(1 - bary[0] - bary[1], bary);
}

vec3 offset_ray(const vec3 p, const vec3 n) {
    const float origin = 1. / 32.;
    const float float_scale = 1. / 65536.;
    const float int_scale = 256.;

    ivec3 of_i = ivec3(n * int_scale) ;
    vec3 p_i = vec3(
        intBitsToFloat(floatBitsToInt(p.x) + ((p.x < 0) ? -of_i.x : of_i.x)),
		intBitsToFloat(floatBitsToInt(p.y) + ((p.y < 0) ? -of_i.y : of_i.y)),
		intBitsToFloat(floatBitsToInt(p.z) + ((p.z < 0) ? -of_i.z : of_i.z))
		);
    return vec3(
    abs(p.x) < origin? p.x + float_scale * n.x: p_i.x,
    abs(p.y) < origin? p.y + float_scale * n.y: p_i.y,
    abs(p.z) < origin? p.z + float_scale * n.z: p_i.z
    );
}



const uint AS_BIND = 0;
const uint STORAGE_BIND = 1;
const uint UNIFORM_BIND = 2;
const uint VERTEX_BIND = 3;
const uint INDEX_BIND = 4;
const uint GEO_BIND = 5;
const uint TEXTURE_BIND = 6;
const uint ACC_BIND = 8;
const uint MAT_BIND = 9;
const uint DLIGHT_BIND = 10;
const uint PLIGHT_BIND = 11;
const uint SKYBOX_BIND = 12;
const uint ANIMATION_BIND = 13;
const uint SKIN_BIND = 14;


const uint RENDER = 0;
const uint HEAT = 1;
const uint INSTANCE = 2;
const uint TRIANGLE = 3;
const uint DISTANCE = 4;
const uint ALBEDO = 5;
const uint METALLIC = 6;
const uint ROUGHNESS = 7;
const uint NORMAL = 8;
const uint TANGENT = 9;
const uint TRANSMISSION = 10;
const uint GEO_ID = 11;


