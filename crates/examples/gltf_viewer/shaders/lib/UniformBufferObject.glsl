#include "PunctualLight.glsl"

struct UniformBufferObject
{
	mat4 ModelView;
	mat4 Projection;
	mat4 ModelViewInverse;
	mat4 ProjectionInverse;

	float Aperture;
	float FocusDistance;
	float HeatmapScale;
	uint TotalNumberOfSamples;

	uint NumberOfSamples;
	uint NumberOfBounces;
	uint RandomSeed;
	bool HasSky;

	bool antialiasing;
	uint mapping;
	uint frame_count;
	uint debug;
};
