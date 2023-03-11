#version 460
#extension GL_EXT_ray_tracing : enable

layout(location = 0) rayPayloadInEXT Payload {
	bool missed;
	bool reflective;
	vec3 hitValue;
	vec3 hitOrigin;
	vec3 hitNormal;
} payload;

void main() {
    payload.missed = true;
    payload.hitValue = vec3(0.392, 0.5843, 0.92941);
}
