#version 460
#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_ray_tracing : require
#include "lib/RayTracingCommons.glsl"
#include "lib/UniformBufferObject.glsl"

layout(binding = UNIFORM_BIND) readonly uniform UniformBufferObjectStruct { UniformBufferObject Camera; };

layout(location = 0) rayPayloadInEXT RayPayload Ray;

void main()
{
	Ray.bary = vec2(0.0);
	if (Camera.HasSky)
	{
		// Sky color
		const float t = 0.5 * (normalize(gl_WorldRayDirectionEXT).y + 1);
		const vec3 skyColor = mix(vec3(1.0), vec3(0.5, 0.7, 1.0), t);
		Ray.hitValue = skyColor;
	}
	else
	{
		Ray.hitValue = vec3(0.01);
	}
	Ray.needScatter = false;
	Ray.t = -1.;
}
