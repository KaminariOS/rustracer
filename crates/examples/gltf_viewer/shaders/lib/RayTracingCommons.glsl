struct RayPayload
{
	vec3 hitValue;
	vec3 hitPoint;
	float t;
	vec3 scatterDirection;
	bool needScatter;
	uint RandomSeed;
	uint instance_id;
	vec2 bary;
};

struct PrimInfo {
    uint v_offset;
    uint i_offset;
    uint material_id;
    uint _padding;
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


// Jenkins's "one at a time" hash function
uint jenkinsHash(uint x) {
	x += x << 10;
	x ^= x >> 6;
	x += x << 3;
	x ^= x >> 11;
	x += x << 15;
	return x;
}


// Maps integers to colors using the hash function (generates pseudo-random colors)
vec3 hashAndColor(uint i) {
	uint hash = jenkinsHash(i);
	float r = ((hash >> 0) & 0xFF) / 255.0f;
	float g = ((hash >> 8) & 0xFF) / 255.0f;
	float b = ((hash >> 16) & 0xFF) / 255.0f;
	return vec3(r, g, b);
}

// Converts unsigned integer into float int range <0; 1) by using 23 most significant bits for mantissa
float uintToFloat(uint x) {
	return uintBitsToFloat(0x3f800000 | (x >> 9)) - 1.0f;
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
