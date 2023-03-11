struct RayPayload
{
	vec3 hitValue;
	vec3 hitPoint;
	float t;
	vec3 scatterDirection;
	bool needScatter;
	uint RandomSeed;
};


vec3 offset_ray(const vec3 p, const vec3 n) {
    const float origin = 1. / 32.;
    const float float_scale = 1. / 65536.;
    const float int_scale = 256.;

    ivec3 of_i = floatBitsToInt(n * int_scale) ;
    vec3 p_i = intBitsToFloat(floatBitsToInt(p) + ivec3(p.x < 0.? -of_i.x : of_i.x, p.y < 0.? -of_i.y: of_i.y, p.z < 0.? -of_i.z: of_i.z)) ;
    return vec3(
    abs(p.x) < origin? p.x + float_scale * n.x: p_i.x,
    abs(p.y) < origin? p.y + float_scale * n.y: p_i.y,
    abs(p.z) < origin? p.z + float_scale * n.z: p_i.z
    );
}
