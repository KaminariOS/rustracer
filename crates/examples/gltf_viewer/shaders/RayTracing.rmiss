#version 460
#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_ray_tracing : require
#include "lib/RayTracingCommons.glsl"
#include "lib/UniformBufferObject.glsl"

layout(binding = UNIFORM_BIND) readonly uniform UniformBufferObjectStruct { UniformBufferObject Camera; };
layout(binding = DLIGHT_BIND) readonly buffer Lights { Light[] lights; };

layout(location = 0) rayPayloadInEXT RayPayload Ray;

void main()
{
	Ray.bary = vec2(0.0);
	vec3 light_acc = vec3(0.);
	for(int i = 0; i < lights.length(); i++) {
		Light li = lights[i];
			float cos = dot(li.transform.xyz, gl_WorldRayDirectionEXT);
			if (cos < 0.) {
				light_acc += -cos * li.color.xyz * li.intensity;
			}
	}
	if (Camera.HasSky)
	{
		// Sky color
		const float t = 0.5 * (normalize(gl_WorldRayDirectionEXT).y + 1);
		const vec3 skyColor = mix(vec3(1.0), vec3(0.5, 0.7, 1.0), t);
		light_acc += skyColor + light_acc;
	} else
	{
		light_acc += vec3(0.01);
	}
	Ray.hitValue = light_acc;
	Ray.needScatter = false;
	Ray.t = -1.;
}
