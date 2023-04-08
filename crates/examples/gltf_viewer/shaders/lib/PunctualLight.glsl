#ifndef VIRTUAL_LIGHT
#define VIRTUAL_LIGHT

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
#endif