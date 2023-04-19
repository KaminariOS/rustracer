#extension GL_EXT_nonuniform_qualifier : require
#ifndef PBR
#define PBR

// https://github.com/boksajak/referencePT
// BRDF types
#define DIFFUSE_TYPE 1
#define SPECULAR_TYPE 2
#define TRANSMISSION_TYPE 3


// Data needed to evaluate BRDF (surface and material properties at given point + configuration of light and normal vectors)
struct BrdfData
{
// Material properties
	vec3 specularF0;
	vec3 diffuseReflectance;
	vec3 specularF90;

// Roughnesses
	float roughness;    //< perceptively linear roughness (artist's input)
	float alpha;        //< linear roughness - often 'alpha' in specular BRDF equations
	float alphaSquared; //< alpha squared - pre-calculated value commonly used in BRDF equations

// Commonly used terms for BRDF evaluation
	vec3 F; //< Fresnel term

// Vectors
	vec3 V; //< Direction to viewer (or opposite direction of incident ray)
	vec3 N; //< Shading normal
	vec3 H; //< Half vector (microfacet normal)
	vec3 L; //< Direction to light (or direction of reflecting ray)
	vec3 Ht; // Transmission angle

	float NdotL;
	float NdotV;

	float LdotH;
	float NdotH;
	float VdotH;

// True when V/L is backfacing wrt. shading normal N
	bool Vbackfacing;
	bool Lbackfacing;
};

// Polynomial approximation by Christophe Schlick
float Schlick(const float cosine, const float refractionIndex)
{
	float r0 = (1 - refractionIndex) / (1 + refractionIndex);
	r0 *= r0;
	return r0 + (1 - r0) * pow(1 - cosine, 5);
}


#define MIN_DIELECTRICS_F0 0.04f
#define PI 3.141592653589f
#define ONE_OVER_PI (1.0f / PI)
#define TWO_PI (2.0f * PI)

// Specify what NDF (GGX or BECKMANN you want to use)
#ifndef MICROFACET_DISTRIBUTION
#define MICROFACET_DISTRIBUTION GGX
//#define MICROFACET_DISTRIBUTION BECKMANN
#endif

//#ifndef DIFFUSE_BRDF
//#define DIFFUSE_BRDF LAMBERTIAN
//#define DIFFUSE_BRDF OREN_NAYAR
//#define DIFFUSE_BRDF DISNEY
#define DIFFUSE_BRDF FROSTBITE
//#define DIFFUSE_BRDF NONE
//#endif


// Select distribution function
//#if MICROFACET_DISTRIBUTION == GGX
#define Microfacet_D GGX_D
//#elif MICROFACET_DISTRIBUTION == BECKMANN
//#define Microfacet_D Beckmann_D
//#endif

// Select G functions (masking/shadowing) depending on selected distribution
#if MICROFACET_DISTRIBUTION == GGX
#define Smith_G_Lambda Smith_G_Lambda_GGX
#elif MICROFACET_DISTRIBUTION == BECKMANN
#define Smith_G_Lambda Smith_G_Lambda_Beckmann_Walter
#endif


#ifndef Smith_G1
// Define version of G1 optimized specifically for selected NDF
#if MICROFACET_DISTRIBUTION == GGX
#define Smith_G1 Smith_G1_GGX
#elif MICROFACET_DISTRIBUTION == BECKMANN
#define Smith_G1 Smith_G1_Beckmann_Walter
#endif
#endif

// Select default specular and diffuse BRDF functions
#if SPECULAR_BRDF == MICROFACET
#define evalSpecular evalMicrofacet
#define sampleSpecular sampleSpecularMicrofacet
#if MICROFACET_DISTRIBUTION == GGX
#define sampleSpecularHalfVector sampleGGXVNDF
#else
#define sampleSpecularHalfVector sampleBeckmannWalter
#endif
//#elif SPECULAR_BRDF == PHONG
//#define evalSpecular evalPhong
//#define sampleSpecular sampleSpecularPhong
//#define sampleSpecularHalfVector samplePhong
//#else
//#define evalSpecular evalVoid
//#define sampleSpecular sampleSpecularVoid
//#define sampleSpecularHalfVector sampleSpecularHalfVectorVoid
#endif

#if MICROFACET_DISTRIBUTION == GGX
#define specularSampleWeight specularSampleWeightGGXVNDF
#define specularPdf sampleGGXVNDFReflectionPdf
#else
//#define specularSampleWeight specularSampleWeightBeckmannWalter
//#define specularPdf sampleBeckmannWalterReflectionPdf
#endif

#define DIFFUSE_BRDF FROSTBITE
//#if DIFFUSE_BRDF == LAMBERTIAN
//#define evalDiffuse evalLambertian
//#define diffuseTerm lambertian
//#elif DIFFUSE_BRDF == OREN_NAYAR
//#define evalDiffuse evalOrenNayar
//#define diffuseTerm orenNayar
//#elif DIFFUSE_BRDF == DISNEY
//#define evalDiffuse evalDisneyDiffuse
//#define diffuseTerm disneyDiffuse
#if DIFFUSE_BRDF == FROSTBITE
#define evalDiffuse evalFrostbiteDisneyDiffuse
#define diffuseTerm frostbiteDisneyDiffuse
//#else
//#define evalDiffuse evalVoid
//#define evalIndirectDiffuse evalIndirectVoid
//#define diffuseTerm none
#endif


// Enable optimized G2 implementation which includes division by specular BRDF denominator (not available for all NDFs, check macro G2_DIVIDED_BY_DENOMINATOR if it was actually used)
#define USE_OPTIMIZED_G2 1

// Enable height correlated version of G2 term. Separable version will be used otherwise
#define USE_HEIGHT_CORRELATED_G2 1

// Enable this to weigh diffuse by Fresnel too, otherwise specular and diffuse will be simply added together
// (this is disabled by default for Frostbite diffuse which is normalized to combine well with GGX Specular BRDF)
#if DIFFUSE_BRDF != FROSTBITE
#define COMBINE_BRDFS_WITH_FRESNEL 1
#endif

#define sampleSpecular sampleSpecularMicrofacet
#define sampleSpecularHalfVector sampleGGXVNDF
#define Smith_G1 Smith_G1_GGX

#define specularSampleWeight specularSampleWeightGGXVNDF
#define specularPdf sampleGGXVNDFReflectionPdf

// BRDF types
#define DIFFUSE_TYPE 1
#define SPECULAR_TYPE 2
#define TRANSMISSION_TYPE 3

const float OURSIDE_IOR = 1.;

struct MaterialBrdf {
	vec3 baseColor;
	float metallic;
	float roughness;
	float ior;
	float transmission;
	bool use_spec;
	float specular_factor;
	vec3 specular_color_factor;
	vec3 dielectricSpecularF0;
	vec3 dielectricSpecularF90;
	vec3 F0;
	vec3 F90;
	vec3 c_diff;
	bool frontFace;

	vec3 attenuation_color;
	float attenuation_distance;
	bool volume;
	float t_diff;
};

void matBuild(inout MaterialBrdf material) {
	// https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_ior/README.md
	// Note that for the default index of refraction ior = 1.5 this term evaluates to dielectricSpecular = 0.04.
	 // MIN_DIELECTRICS_F0 = 0.04
	float factor = (material.ior - OURSIDE_IOR) / (material.ior + OURSIDE_IOR);
	material.dielectricSpecularF0 = min(factor * factor * material.specular_color_factor, vec3(1.0)) *
	material.specular_factor;
	material.dielectricSpecularF90 = material.specular_color_factor;
	material.F0 = mix(material.dielectricSpecularF0, material.baseColor, material.metallic);
	material.F90 = mix(material.dielectricSpecularF90, vec3(1.), material.metallic);
	material.c_diff = mix(material.baseColor, vec3(0.0), material.metallic);
}


float rsqrt(float x) { return inversesqrt(x); }


float saturate(float x) { return clamp(x, 0.0f, 1.0f); }

vec3 baseColorToSpecularF0(const MaterialBrdf material, vec3 baseColor, float metalness) {

	vec3 dielectricSpecularF0 = min(MIN_DIELECTRICS_F0 * material.specular_color_factor, vec3(1.0)) *
	material.specular_factor;
	return mix(dielectricSpecularF0, baseColor, metalness);
}

float luminance(vec3 rgb)
{
	return dot(rgb, vec3(0.2126f, 0.7152f, 0.0722f));
}

// Schlick's approximation to Fresnel term
// f90 should be 1.0, except for the trick used by Schuler (see 'shadowedF90' function)
vec3 evalFresnelSchlick(vec3 f0, float f90, float NdotS)
{
	return f0 + (f90 - f0) * pow(1.0f - NdotS, 5.0f);
}
struct BRDF {
	float specular;
	float diffuse;
	float transmission;
};


vec3 evalFresnel(vec3 f0, float f90, float NdotS)
{
	// Default is Schlick's approximation
	return evalFresnelSchlick(f0, f90, NdotS);
}

// Function to calculate 'a' parameter for lambda functions needed in Smith G term
// This is a version for shape invariant (isotropic) NDFs
// Note: makse sure NdotS is not negative
float Smith_G_a(float alpha, float NdotS) {
	return NdotS / (max(0.00001f, alpha) * sqrt(1.0f - min(0.99999f, NdotS * NdotS)));
}

// Lambda function for Smith G term derived for GGX distribution
float Smith_G_Lambda_GGX(float a) {
	return (-1.0f + sqrt(1.0f + (1.0f / (a * a)))) * 0.5f;
}
// Smith G2 term (masking-shadowing function)
// Height correlated version - non-optimized, uses G_Lambda functions for selected NDF
float Smith_G2_Height_Correlated(float alpha, float NdotL, float NdotV) {
	float aL = Smith_G_a(alpha, NdotL);
	float aV = Smith_G_a(alpha, NdotV);
	return 1.0f / (1.0f + Smith_G_Lambda(aL) + Smith_G_Lambda(aV));
}
// Evaluates G2 for selected configuration (GGX/Beckmann, optimized/non-optimized, separable/height-correlated)
// Note that some paths aren't optimized too much...
// Also note that when USE_OPTIMIZED_G2 is specified, returned value will be: G2 / (4 * NdotL * NdotV) if GG-X is selected
float Smith_G2(float alpha, float alphaSquared, float NdotL, float NdotV) {

//	#if USE_OPTIMIZED_G2 && (MICROFACET_DISTRIBUTION == GGX)
//	#if USE_HEIGHT_CORRELATED_G2
//	#define G2_DIVIDED_BY_DENOMINATOR 1
//	return Smith_G2_Height_Correlated_GGX_Lagarde(alphaSquared, NdotL, NdotV);
//	#else
//	#define G2_DIVIDED_BY_DENOMINATOR 1
//	return Smith_G2_Separable_GGX_Lagarde(alphaSquared, NdotL, NdotV);
//	#endif
//	#else
//	#if USE_HEIGHT_CORRELATED_G2
	return Smith_G2_Height_Correlated(alpha, NdotL, NdotV);
//	#else
//	return Smith_G2_Separable(alpha, NdotL, NdotV);
//	#endif
//	#endif
}


float GGX_D(float alphaSquared, float NdotH) {
	float b = ((alphaSquared - 1.0f) * NdotH * NdotH + 1.0f);
	return alphaSquared / (PI * b * b);
}

// Evaluates microfacet specular BRDF
vec3 evalMicrofacet(const BrdfData data) {

	float D = Microfacet_D(max(0.00001f, data.alphaSquared), data.NdotH);
	float G2 = Smith_G2(data.alpha, data.alphaSquared, data.NdotL, data.NdotV);
	//float3 F = evalFresnel(data.specularF0, shadowedF90(data.specularF0), data.VdotH); //< Unused, F is precomputed already

//	#if G2_DIVIDED_BY_DENOMINATOR
//	return data.F * (G2 * D * data.NdotL);
//	#else
	return ((data.F * G2 * D) / (4.0f * data.NdotL * data.NdotV)) * data.NdotL;
//	#endif
}



// Attenuates F90 for very low F0 values
// Source: "An efficient and Physically Plausible Real-Time Shading Model" in ShaderX7 by Schuler
// Also see section "Overbright highlights" in Hoffman's 2010 "Crafting Physically Motivated Shading Models for Game Development" for discussion
// IMPORTANT: Note that when F0 is calculated using metalness, it's value is never less than MIN_DIELECTRICS_F0, and therefore,
// this adjustment has no effect. To be effective, F0 must be authored separately, or calculated in different way. See main text for discussion.
float shadowedF90(const vec3 F90
//, vec3 F0
) {
	// This scaler value is somewhat arbitrary, Schuler used 60 in his article. In here, we derive it from MIN_DIELECTRICS_F0 so
	// that it takes effect for any reflectance lower than least reflective dielectrics
	//const float t = 60.0f;
//	const float t = (1.0f / MIN_DIELECTRICS_F0);
//	vec3 dielectricSpecularF90 = mat.specular_color_factor;
//	vec3 F90 = mix(dielectricSpecularF90, vec3(1.), mat.metallic);
	return min(1.0f, luminance(F90));
}

BRDF getBrdfProbability(MaterialBrdf mat, vec3 V, vec3 shadingNormal) {

	// Evaluate Fresnel term using the shading normal
	// Note: we use the shading normal instead of the microfacet normal (half-vector) for Fresnel term here. That's suboptimal for rough surfaces at grazing angles, but half-vector is yet unknown at this point
	float specularF0 = luminance(mat.F0);
	float diffuseReflectance = luminance(mat.c_diff);
	float Fresnel = saturate(luminance(evalFresnel(vec3(specularF0), shadowedF90(mat.F90), max(0.0f, dot(V, shadingNormal)))));

	// Approximate relative contribution of BRDFs using the Fresnel term
	float specular = Fresnel * mat.specular_factor;
	float penetration = diffuseReflectance * (1.0f - mat.specular_factor * Fresnel);
	float diffuse = penetration * (1. - mat.transmission);  //< If diffuse term is weighted by Fresnel, apply it here as well
	float transmission = penetration * mat.transmission;

	// Return probability of selecting specular BRDF over diffuse BRDF
	float sum = max(0.0001f, (specular + diffuse + transmission));
	float p = specular / sum ;
	const float min = 0.001;
	const float max = 0.9;
	p = clamp(p, min, max);
	float d = (1 - p) * (1 - mat.transmission);
	float t = (1 - p) * mat.transmission;
//	d = min(1., d);
//	t = clamp(t, min, max);
	sum = p + d + t;
	p /= sum;
	d /= sum;
	t /= sum;
	BRDF brdf;
	brdf.specular = p;
	brdf.diffuse = d;
	brdf.transmission = t;
	// Clamp probability to avoid undersampling of less prominent BRDF
	return brdf;
}

// Calculates rotation quaternion from input vector to the vector (0, 0, 1)
// Input vector must be normalized!
vec4 getRotationToZAxis(vec3 input_vec) {

	// Handle special case when input is exact or near opposite of (0, 0, 1)
	if (input_vec.z < -0.99999f) return vec4(1.0f, 0.0f, 0.0f, 0.0f);

	return normalize(vec4(input_vec.y, -input_vec.x, 0.0f, 1.0f + input_vec.z));
}


// Optimized point rotation using quaternion
// Source: https://gamedev.stackexchange.com/questions/28395/rotating-vector3-by-a-quaternion
vec3 rotatePoint(vec4 q, vec3 v) {
	const vec3 qAxis = vec3(q.x, q.y, q.z);
	return 2.0f * dot(qAxis, v) * qAxis + (q.w * q.w - dot(qAxis, qAxis)) * v + 2.0f * q.w * cross(qAxis, v);
}


// Samples a direction within a hemisphere oriented along +Z axis with a cosine-weighted distribution
// Source: "Sampling Transformations Zoo" in Ray Tracing Gems by Shirley et al.
vec3 sampleHemisphere(vec2 u, inout float pdf) {

	float a = sqrt(u.x);
	float b = TWO_PI * u.y;

	vec3 result = vec3(
	a * cos(b),
	a * sin(b),
	sqrt(1.0f - u.x));

	pdf = result.z * ONE_OVER_PI;

	return result;
}

vec3 sampleHemisphere(vec2 u) {
	float pdf;
	return sampleHemisphere(u, pdf);
}




// -------------------------------------------------------------------------
//    Microfacet model
// -------------------------------------------------------------------------

// Samples a microfacet normal for the GGX distribution using VNDF method.
// Source: "Sampling the GGX Distribution of Visible Normals" by Heitz
// See also https://hal.inria.fr/hal-00996995v1/document and http://jcgt.org/published/0007/04/01/
// Random variables 'u' must be in <0;1) interval
// PDF is 'G1(NdotV) * D'
vec3 sampleGGXVNDF(vec3 Ve, vec2 alpha2D, vec2 u) {

	// Section 3.2: transforming the view direction to the hemisphere configuration
	vec3 Vh = normalize(vec3(alpha2D.x * Ve.x, alpha2D.y * Ve.y, Ve.z));

	// Section 4.1: orthonormal basis (with special case if cross product is zero)
	float lensq = Vh.x * Vh.x + Vh.y * Vh.y;
	vec3 T1 = lensq > 0.0f ? vec3(-Vh.y, Vh.x, 0.0f) * rsqrt(lensq) : vec3(1.0f, 0.0f, 0.0f);
	vec3 T2 = cross(Vh, T1);

	// Section 4.2: parameterization of the projected area
	float r = sqrt(u.x);
	float phi = TWO_PI * u.y;
	float t1 = r * cos(phi);
	float t2 = r * sin(phi);
	float s = 0.5f * (1.0f + Vh.z);
	t2 = mix(sqrt(1.0f - t1 * t1), t2, s);

	// Section 4.3: reprojection onto hemisphere
	vec3 Nh = t1 * T1 + t2 * T2 + sqrt(max(0.0f, 1.0f - t1 * t1 - t2 * t2)) * Vh;

	// Section 3.4: transforming the normal back to the ellipsoid configuration
	return normalize(vec3(alpha2D.x * Nh.x, alpha2D.y * Nh.y, max(0.0f, Nh.z)));
}


// Smith G1 term (masking function) optimized for GGX distribution (by substituting G_Lambda_GGX into G1)
float Smith_G1_GGX(float a) {
	float a2 = a * a;
	return 2.0f / (sqrt((a2 + 1.0f) / a2) + 1.0f);
}

// Smith G1 term (masking function) further optimized for GGX distribution (by substituting G_a into G1_GGX)
float Smith_G1_GGX(float alpha, float NdotS, float alphaSquared, float NdotSSquared) {
	return 2.0f / (sqrt(((alphaSquared * (1.0f - NdotSSquared)) + NdotSSquared) / NdotSSquared) + 1.0f);
}

// PDF of sampling a reflection vector L using 'sampleGGXVNDF'.
// Note that PDF of sampling given microfacet normal is (G1 * D) when vectors are in local space (in the hemisphere around shading normal).
// Remaining terms (1.0f / (4.0f * NdotV)) are specific for reflection case, and come from multiplying PDF by jacobian of reflection operator
float sampleGGXVNDFReflectionPdf(float alpha, float alphaSquared, float NdotH, float NdotV, float LdotH) {
	NdotH = max(0.00001f, NdotH);
	NdotV = max(0.00001f, NdotV);
	return (GGX_D(max(0.00001f, alphaSquared), NdotH) * Smith_G1_GGX(alpha, NdotV, alphaSquared, NdotV * NdotV)) / (4.0f * NdotV);
}


// Smith G2 term (masking-shadowing function)
// Separable version assuming independent (uncorrelated) masking and shadowing, uses G1 functions for selected NDF
float Smith_G2_Separable(float alpha, float NdotL, float NdotV) {
	float aL = Smith_G_a(alpha, NdotL);
	float aV = Smith_G_a(alpha, NdotV);
	return Smith_G1(aL) * Smith_G1(aV);
}

// A fraction G2/G1 where G2 is height correlated can be expressed using only G1 terms
// Source: "Implementing a Simple Anisotropic Rough Diffuse Material with Stochastic Evaluation", Appendix A by Heitz & Dupuy
float Smith_G2_Over_G1_Height_Correlated(float alpha, float alphaSquared, float NdotL, float NdotV) {
	float G1V = Smith_G1(alpha, NdotV, alphaSquared, NdotV * NdotV);
	float G1L = Smith_G1(alpha, NdotL, alphaSquared, NdotL * NdotL);
	return G1L / (G1V + G1L - G1V * G1L);
}

// Weight for the reflection ray sampled from GGX distribution using VNDF method
float specularSampleWeightGGXVNDF(float alpha, float alphaSquared, float NdotL, float NdotV, float HdotL, float NdotH) {
	//    #if USE_HEIGHT_CORRELATED_G2
	return Smith_G2_Over_G1_Height_Correlated(alpha, alphaSquared, NdotL, NdotV);
	//    #else
	//    return Smith_G1_GGX(alpha, NdotL, alphaSquared, NdotL * NdotL);
	//    #endif
}
// Frostbite's version of Disney diffuse with energy normalization.
// Source: "Moving Frostbite to Physically Based Rendering" by Lagarde & de Rousiers
float frostbiteDisneyDiffuse(const BrdfData data) {
	float energyBias = 0.5f * data.roughness;
	float energyFactor = mix(1.0f, 1.0f / 1.51f, data.roughness);

	float FD90MinusOne = energyBias + 2.0 * data.LdotH * data.LdotH * data.roughness - 1.0f;

	float FDL = 1.0f + (FD90MinusOne * pow(1.0f - data.NdotL, 5.0f));
	float FDV = 1.0f + (FD90MinusOne * pow(1.0f - data.NdotV, 5.0f));

	return FDL * FDV * energyFactor;
}

float Smith_Vt(const BrdfData data) {
	float NdotL = data.NdotL;
	float NdotV = data.NdotV;
	float HtL = dot(data.Ht, data.L);
	float HtV = dot(data.Ht, data.V);
	float a_square = data.alphaSquared;
	float one_minus_a_s = 1 - a_square;
	float first = (HtL / NdotL) / (abs(NdotL) + sqrt(a_square + one_minus_a_s * NdotL * NdotL));
	float second = (HtV  / NdotV) / (a_square + sqrt(a_square + one_minus_a_s * NdotV * NdotV));
	return first * second;
}

float specular_btdf(const BrdfData data) {
	float Dt = Microfacet_D(data.alphaSquared, dot(data.N, data.Ht));
	float Vt = Smith_Vt(data);
	return Vt * Dt;
}

vec3 evalFrostbiteDisneyDiffuse(const BrdfData data) {
	return data.diffuseReflectance * (frostbiteDisneyDiffuse(data) * ONE_OVER_PI * data.NdotL);
}

// -------------------------------------------------------------------------
//    Combined BRDF
// -------------------------------------------------------------------------

// Precalculates commonly used terms in BRDF evaluation
// Clamps around dot products prevent NaNs and ensure numerical stability, but make sure to
// correctly ignore rays outside of the sampling hemisphere, by using 'Vbackfacing' and 'Lbackfacing' flags
BrdfData prepareBRDFData(vec3 N, vec3 L, vec3 V, MaterialBrdf material) {
	BrdfData data;

	// Evaluate VNHL vectors
	data.V = V;
	data.N = N;
	data.H = normalize(L + V);
	data.L = L;
	data.Ht = normalize(V - 2 * dot(N, L) * N + L);

	float NdotL = dot(N, L);
	float NdotV = dot(N, V);
	data.Vbackfacing = (NdotV <= 0.0f);
	data.Lbackfacing = (NdotL <= 0.0f);

	// Clamp NdotS to prevent numerical instability. Assume vectors below the hemisphere will be filtered using 'Vbackfacing' and 'Lbackfacing' flags
	data.NdotL = min(max(0.00001f, NdotL), 1.0f);
	data.NdotV = min(max(0.00001f, NdotV), 1.0f);

	data.LdotH = saturate(dot(L, data.H));
	data.NdotH = saturate(dot(N, data.H));
	data.VdotH = saturate(dot(V, data.H));

	// Unpack material properties
	data.specularF0 = material.F0;
	data.specularF90 = material.F90;
	data.diffuseReflectance = material.c_diff;

	// Unpack 'perceptively linear' -> 'linear' -> 'squared' roughness
	data.roughness = material.roughness;
	data.alpha = material.roughness * material.roughness;
	data.alphaSquared = data.alpha * data.alpha;

	// Pre-calculate some more BRDF terms
	data.F = evalFresnel(data.specularF0, shadowedF90(material.F90), data.VdotH);

	return data;
}


// -------------------------------------------------------------------------
//    Lambert
// -------------------------------------------------------------------------

//float lambertian(const BrdfData data) {
//	return 1.0f;
//}
//
//vec3 evalLambertian(const BrdfData data) {
//	return data.diffuseReflectance * (ONE_OVER_PI * data.NdotL);
//}

// Returns the quaternion with inverted rotation
vec4 invertRotation(vec4 q)
{
	return vec4(-q.x, -q.y, -q.z, q.w);
}

// Samples a reflection ray from the rough surface using selected microfacet distribution and sampling method
// Resulting weight includes multiplication by cosine (NdotL) term
vec3 sampleSpecularMicrofacet(vec3 Vlocal, float alpha, float alphaSquared, vec3 specularF0, vec2 u, inout vec3 weight, vec3 specularF90) {

	// Sample a microfacet normal (H) in local space
	vec3 Hlocal;
	if (alpha == 0.0f) {
		// Fast path for zero roughness (perfect reflection), also prevents NaNs appearing due to divisions by zeroes
		Hlocal = vec3(0.0f, 0.0f, 1.0f);
	} else {
		// For non-zero roughness, this calls VNDF sampling for GG-X distribution or Walter's sampling for Beckmann distribution
		Hlocal = sampleSpecularHalfVector(Vlocal, vec2(alpha, alpha), u);
	}

	// Reflect view direction to obtain light vector
	vec3 Llocal = reflect(-Vlocal, Hlocal);

	// Note: HdotL is same as HdotV here
	// Clamp dot products here to small value to prevent numerical instability. Assume that rays incident from below the hemisphere have been filtered
	float HdotL = max(0.00001f, min(1.0f, dot(Hlocal, Llocal)));
	const vec3 Nlocal = vec3(0.0f, 0.0f, 1.0f);
	float NdotL = max(0.00001f, min(1.0f, dot(Nlocal, Llocal)));
	float NdotV = max(0.00001f, min(1.0f, dot(Nlocal, Vlocal)));
	float NdotH = max(0.00001f, min(1.0f, dot(Nlocal, Hlocal)));
	vec3 F = evalFresnel(specularF0, shadowedF90(specularF90), HdotL);

	// Calculate weight of the sample specific for selected sampling method
	// (this is microfacet BRDF divided by PDF of sampling method - notice how most terms cancel out)
	weight = F * specularSampleWeight(alpha, alphaSquared, NdotL, NdotV, HdotL, NdotH);
	//    weight = vec3(1.);
	return Llocal;
}

// Samples a reflection ray from the rough surface using selected microfacet distribution and sampling method
// Resulting weight includes multiplication by cosine (NdotL) term
vec3 sampleSpecularMicrofacetRefract(vec3 Vlocal, float alpha, float alphaSquared, vec3 specularF0, vec2 u, inout vec3 weight, vec3 specularF90, MaterialBrdf material) {
	// Sample a microfacet normal (H) in local space
	vec3 Hlocal;
	if (alpha == 0.0f) {
		// Fast path for zero roughness (perfect reflection), also prevents NaNs appearing due to divisions by zeroes
		Hlocal = vec3(0.0f, 0.0f, 1.0f);
	} else {
		// For non-zero roughness, this calls VNDF sampling for GG-X distribution or Walter's sampling for Beckmann distribution
		Hlocal = sampleSpecularHalfVector(Vlocal, vec2(alpha, alpha), u);
	}
	const float refraction_ratio = material.frontFace ? 1 / material.ior: material.ior;
	// Reflect view direction to obtain light vector
	const vec3 Llocal = refract(-Vlocal, Hlocal, refraction_ratio);
	// Note: HdotL is same as HdotV here
	// Clamp dot products here to small value to prevent numerical instability. Assume that rays incident from below the hemisphere have been filtered
	float HdotL = max(0.00001f, min(1.0f, dot(Hlocal, Llocal)));
	const vec3 Nlocal = vec3(0.0f, 0.0f, 1.0f);
	float NdotL = max(0.00001f, min(1.0f, dot(Nlocal, Llocal)));
	float NdotV = max(0.00001f, min(1.0f, dot(Nlocal, Vlocal)));
	float NdotH = max(0.00001f, min(1.0f, dot(Nlocal, Hlocal)));
	vec3 F = evalFresnel(specularF0, shadowedF90(specularF90), HdotL);

	// Calculate weight of the sample specific for selected sampling method
	// (this is microfacet BRDF divided by PDF of sampling method - notice how most terms cancel out)
	weight = F * specularSampleWeight(alpha, alphaSquared, NdotL, NdotV, HdotL, NdotH);
	//    weight = vec3(1.);
	return Llocal;
}

bool evalIndirectCombinedBRDF(vec2 u, vec3 shadingNormal, vec3 geometryNormal, vec3 V,
MaterialBrdf material,
const uint brdfType,
inout vec3 rayDirection,
inout vec3 sampleWeight,
inout float volume_dis
) {
	if (dot(geometryNormal, V) < 0.0f) return false;
	vec4 qRotationToZ = getRotationToZAxis(shadingNormal);
	vec3 Vlocal = rotatePoint(qRotationToZ, V);
	const vec3 Nlocal = vec3(0.0f, 0.0f, 1.0f);

	vec3 rayDirectionLocal = vec3(0.0f, 0.0f, 0.0f);
	if (brdfType == DIFFUSE_TYPE) {
		rayDirectionLocal = sampleHemisphere(u);
		const BrdfData data =
		prepareBRDFData(Nlocal, rayDirectionLocal, Vlocal, material);

		// Function 'diffuseTerm' is predivided by PDF of sampling the cosine weighted hemisphere
		sampleWeight = (1 - material.specular_factor * data.F) * data.diffuseReflectance * diffuseTerm(data);
		sampleWeight *= (1. - material.transmission);
		//        sampleWeight = data.diffuseReflectance * lambertian(data);

		//        #if COMBINE_BRDFS_WITH_FRESNEL
		// Sample a half-vector of specular BRDF. Note that we're reusing random variable 'u' here, but correctly it should be an new independent random number
		//        vec3 Hspecular = sampleSpecularHalfVector(Vlocal, vec2(data.alpha, data.alpha), u);
		//
		//        // Clamp HdotL to small value to prevent numerical instability. Assume that rays incident from below the hemisphere have been filtered
		//        float VdotH = max(0.00001f, min(1.0f, dot(Vlocal, Hspecular)));
		//        sampleWeight *= (vec3(1.0f) - evalFresnel(data.specularF0, shadowedF90(data.specularF0), VdotH));
		//        #endif
	}
	else if (brdfType == SPECULAR_TYPE) {
		const BrdfData data = prepareBRDFData(Nlocal, vec3(0.0f, 0.0f, 1.0f) /* unused L vector */, Vlocal, material);
		rayDirectionLocal = sampleSpecular(Vlocal, data.alpha, data.alphaSquared, data.specularF0, u, sampleWeight, data.specularF90);
//		rayDirectionLocal = reflect(-Vlocal, Nlocal);
//		vec3 F = evalFresnel(material.F0, shadowedF90(material.F90), dot(rayDirectionLocal, Nlocal));
//		sampleWeight = F;
		sampleWeight *= material.specular_factor;
	} else if (brdfType == TRANSMISSION_TYPE) {
		if (material.volume) {
			const float refraction_ratio = material.frontFace ? 1 / material.ior: material.ior;
			const vec3 refracted = refract(-Vlocal, Nlocal, refraction_ratio);
			if (refracted == vec3(0.)) {
				sampleWeight = vec3(0.);
				return false;
			}
			rayDirectionLocal = refracted;
		} else {
			rayDirectionLocal = -Vlocal;
		}
		const BrdfData data = prepareBRDFData(Nlocal, rayDirectionLocal, Vlocal, material);
		float NdotL = dot(geometryNormal, rayDirection) ;
		sampleWeight = max(vec3(0),
		data.diffuseReflectance
		* material.transmission
//		* specular_btdf(data)
		)
		;
		if (!material.frontFace && material.volume) {
//			float dis = material.t_diff;
			float dis = volume_dis;
			volume_dis = -1;
			vec3 sigma = log(material.attenuation_color) / material.attenuation_distance;
			vec3 attenuation = exp(sigma * dis);
			sampleWeight *= min(attenuation, vec3(1.));
		}
//		sampleWeight = vec3(1.);
	}

	// Prevent tracing direction with no contribution
	if (luminance(sampleWeight) == 0.0f) return false;

	// Transform sampled direction Llocal back to V vector space
	rayDirection = normalize(rotatePoint(invertRotation(qRotationToZ), rayDirectionLocal));

	// Prevent tracing direction "under" the hemisphere (behind the triangle)
	float NdotL = dot(geometryNormal, rayDirection) ;

	return true;
}

vec3 evalCombinedBRDF(vec3 N, vec3 L, vec3 V, MaterialBrdf material) {

	// Prepare data needed for BRDF evaluation - unpack material properties and evaluate commonly used terms (e.g. Fresnel, NdotL, ...)
	const BrdfData data = prepareBRDFData(N, L, V, material);

	// Ignore V and L rays "below" the hemisphere
	if (data.Vbackfacing || data.Lbackfacing) return vec3(0.0f, 0.0f, 0.0f);

	// Eval specular and diffuse BRDFs
	vec3 specular = evalSpecular(data);
	vec3 diffuse = evalDiffuse(data);

	// Combine specular and diffuse layers
	return diffuse + specular;
}

const int E_DIFFUSE = 0x00001;
const int E_DELTA = 0x00002;
const int E_REFLECTION = 0x00004;
const int E_TRANSMISSION = 0x00008;
const int E_COATING = 0x00010;
const int E_STRAIGHT = 0x00020;
const int E_OPAQUE_DIELECTRIC = 0x00040;
const int E_TRANSPARENT_DIELECTRIC = 0x00080;
const int E_METAL = 0x00100;

vec3 eval_pbr(const in MaterialBrdf material, vec3 V, vec3 L, vec3 shadingNormal) {
	vec3 H = normalize(V + L);
	vec4 qRotationToZ = getRotationToZAxis(shadingNormal);
	vec3 Vlocal = rotatePoint(qRotationToZ, V);
	vec3 Llocal = rotatePoint(qRotationToZ, L);
	const vec3 Nlocal = vec3(0.0f, 0.0f, 1.0f);
	// diffuse
	// opaque diecletric
	// metal
	// transparen dielectric

	return vec3(1.);
}

float eval_pdf(const in BrdfData data) {
	return 0.;
}

void select_bsdf(inout BrdfData data, inout RngStateType rng_state) {

}

vec3 sample_pbr(inout BrdfData data, inout float bsdf_over_pdf, out float pdf) {
	return vec3(1.);
}

//void select_bsdf()

#endif