struct RayPayload
{
	vec3 hitValue;
	vec3 hitPoint;
	float t;
	vec3 scatterDirection;
	bool needScatter;
	uint RandomSeed;
	vec3 emittance;
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

const uint DIRECT_LIGHT = 0;
const uint POINT_LIGHT = 1;
const uint SPOT_LIGHT = 2;

struct Light {
    vec4 color;
    vec4 transform;
    uint kind;
    float range;
    float intensity;
    float _padding;
};

vec3 getLightIntensityAtPoint(Light light, float distance) {
    vec3 color = light.intensity * light.color.rgb;
	if (light.kind == POINT_LIGHT) {
		// Cem Yuksel's improved attenuation avoiding singularity at distance=0
		// Source: http://www.cemyuksel.com/research/pointlightattenuation/
		const float radius = 0.5f; //< We hardcode radius at 0.5, but this should be a light parameter
		const float radiusSquared = radius * radius;
		const float distanceSquared = distance * distance;
		const float attenuation = 2.0f / (distanceSquared + radiusSquared + distance * sqrt(distanceSquared + radiusSquared));
		return color * attenuation;
	} else {
	    return color;
	}
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
