// Ray tracing Gems 2, Chapter 3
const float tMin = 0.001;
const float tMax = 10000.0;
struct RayInfo {
    vec3 pos;
    float min;
    vec3 dir;
    float max;
};