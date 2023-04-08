#include "Random.glsl"
#include "PunctualLight.glsl"
#define STANDARD_RAY_INDEX 0
#define SHADOW_RAY_INDEX 1
#define SHADOW_RAY_IN_RIS 0

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
	vec3 color;
	vec2 uvs;
	uint material_index;
};

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

const float tMin = 0.001;
const float tMax = 10000.0;

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


const uint RENDER = 0;
const uint HEAT = 1;
const uint INSTANCE = 2;
const uint TRIANGLE = 3;
const uint DISTANCE = 4;
const uint ALBEDO = 5;

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
