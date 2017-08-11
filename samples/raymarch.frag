#version 140

//// Output variables
out vec4 color;

//// Input variables
uniform float time;
uniform vec2 resolution;

//// Global variables

vec2 uv;

//// Definitions

#define CAMERA_SPEED 0.5

#define MAX_STEPS 64.0
#define EPSILON 0.0001

#define SHADOWS 1
#define SHADOW_THRESHOLD 0.01
#define SOFT_SHADOWS 0
#define SHADOW_SOFTNESS 1.0

#define LIGHT_INTENSITY 25.0
#define AMBIENT_LIGHT 0.05

#define REFLECTIONS 0

#define FOG 1
#define FOG_DISTANCE 50.0
#define FOG_COLOR vec3(0.8, 0.9, 1.0)

#define MATERIAL_LIGHTGRAY vec3(0.9, 0.9, 0.9)
#define MATERIAL_GREEN vec3(0.258824, 0.956863, 0.607843)
#define MATERIAL_BLUE vec3(0.258824, 0.52549, 0.956863)
#define MATERIAL_PURPLE vec3(0.878431, 0.4, 1.0)
#define MATERIAL_YELLOW vec3(1.0, 0.843137, 0.0)

//// Structs

struct MapInfo {
    vec3 color; // Color of the point we got info from
    float hit; // Distance of the point we got info from to the map
};

struct Ray {
    vec3 origin; // Origin of the ray
    vec3 direction; // Ray's direction

    vec3 target; // Where the ray is aiming at

    float length; // Current position along the ray
    vec3 position; // Just so we don't have to calculate it multiple times ¯\_(ツ)_/¯
};

//// Ray initializer

Ray initRay(vec3 origin, vec3 target) {
    Ray ray;

    // Init ray values
    ray.direction = normalize(target - origin); // Calculate ray direction

    ray.origin = origin + ray.direction * EPSILON; // Start a bit away from the origin so we don't hit whatever is at the start

    ray.target = target;
    ray.position = origin;

    return ray;
}

//// Distance functions

MapInfo plane(vec3 origin, vec3 color) {
    return MapInfo(color, -origin.y);
}

MapInfo sphere(vec3 origin, float rad, vec3 color) {
    return MapInfo(color, length(origin) - rad);
}

MapInfo box(vec3 origin, vec3 size, vec3 color) {
    return MapInfo(color, length(max(abs(origin) - size, 0.0)));
}

//// Distance operations

MapInfo opUnion(MapInfo o1, MapInfo o2) {
    return o1.hit < o2.hit ? o1 : o2; // min(o1, o2)
}

MapInfo opSubtract(MapInfo o1, MapInfo o2) {
    return -o1.hit > o2.hit ? MapInfo(o1.color, -o1.hit) : o2; // max(-o1, o2)
}

MapInfo opIntersect(MapInfo o1, MapInfo o2) {
    return o1.hit > o2.hit ? o1 : o2; // max(o1, o2)
}

//// The main mapping function

MapInfo map(vec3 origin) {
    MapInfo mapInfo = plane(origin, MATERIAL_LIGHTGRAY); // red plane at origin
    mapInfo = opUnion(mapInfo, sphere(origin + vec3(1.5, 1.0, 1.5), 1.0, MATERIAL_BLUE)); // blue sphere at 1.5, 1.0, 1.5 with radius 1.0
    mapInfo = opUnion(mapInfo, box(origin + vec3(-1.5, 1.0, 1.5), vec3(1.0), MATERIAL_GREEN)); // green box at -1.5, 1.0, 1.5 with size 1.0 x 1.0 x 1.0
    mapInfo = opUnion(mapInfo, opSubtract( // subtract sphere from box
                sphere(origin + vec3(-1.5, 1.0, -1.5), 1.2, MATERIAL_PURPLE),  // purple sphere at -1.5, 1.0, -1.5 with radius 1.2
                box(origin + vec3(-1.5, 1.0, -1.5), vec3(1.0), MATERIAL_PURPLE))); // purple box at -1.5, 1.0, -1.5 with size 1.0 x 1.0 x 1.0
    mapInfo = opUnion(mapInfo, opIntersect( // intersect sphere with box
                box(origin + vec3(1.5, 1.0, -1.5), vec3(1.0), MATERIAL_YELLOW), // yellow box at 1.5, 1.0, -1.5 with size 1.0 x 1.0 x 1.0
                sphere(origin + vec3(1.5, 1.0, -1.5), 1.2, MATERIAL_YELLOW))); // yellow sphere at 1.5, 1.0, -1.5 with radius 1.2
    return mapInfo;
}

//// Raymarching functions

vec3 calcNormal(vec3 position) {
    // Step around the point and see how far we are from the map at each position
    // and do some math to figure out what the normal is
    vec2 eps = vec2(0.0, EPSILON);
    return normalize(vec3(
                map(position + eps.yxx).hit - map(position - eps.yxx).hit,
                map(position + eps.xyx).hit - map(position - eps.xyx).hit,
                map(position + eps.xxy).hit - map(position - eps.xxy).hit));
}

MapInfo trace(inout Ray ray) {
    bool bent = false;
    for(float i = 0.0; i < MAX_STEPS; i++) {
        MapInfo mapInfo = map(ray.position); // Get info about our position in relation to the map
        if(mapInfo.hit < EPSILON) return mapInfo; // If we hit something, return the info about our position on the map

        ray.position += ray.direction * mapInfo.hit; // Step forward along the ray, as far as our distance to the map
    }
    return MapInfo(vec3(0.0, 0.0, 0.0), 1.0); // Return black if we didn't hit anything
}

float softshadow(inout Ray ray, float softness) {
    // While we're not past the target, do the stuff
    // Subtract EPSILON * 2 so we don't get close enough to the original object to trigger the shadow
    float penumbra = 1.0;
    while(distance(ray.origin, ray.position) < distance(ray.origin, ray.target) - EPSILON * 2.0) {
        MapInfo mapInfo = map(ray.position); // Get info about our position in relation to the map
        if(mapInfo.hit < EPSILON) return 0.0; // If we hit something, make the color black (shadow)

        #if SOFT_SHADOWS == 1
        //TODO: fix soft shadows
        penumbra = min(penumbra, softness * mapInfo.hit / distance(ray.origin, ray.position));
        #endif

        if(mapInfo.hit > SHADOW_THRESHOLD) ray.position += ray.direction * mapInfo.hit;
        else ray.position += ray.direction * SHADOW_THRESHOLD; // Move a bit closer to the target
    }
    return penumbra; // If we don't hit anything, the point is not in shadow so the shadow multiplier is 1.0
}

//// Main function

void main() {
    //// Setup the viewport
    uv = gl_FragCoord.xy / resolution.xy * 2.0 - 1.0; // Screen coords go from -1.0 to 1.0
    uv.x *= resolution.x / resolution.y; // Account for screen ratio

    //// Setup the camera
    // Init camera 5.0 up, rotating in a circle, looking at 0.0, 0.0, 0.0
    // Axes are flipped here I guess? Note the -vec3s
    Ray cameraRay = initRay(-vec3(sin(time * CAMERA_SPEED) * 5.0, 5.0, cos(time * CAMERA_SPEED) * 5.0), -vec3(0.0, 0.0, 0.0));
    // Convert to screen coords
    vec3 globalUp = vec3(0.0, 1.0, 0.0); // Global up direction
    vec3 cameraRight = normalize(cross(globalUp, cameraRay.origin)); // Right direction in relation to the camera
    vec3 cameraUp = cross(cameraRay.direction, cameraRight); // Up direction in relation to the camera
    // Set the camera ray direction to point from the screen coordinate to our target
    cameraRay.direction = normalize(cameraRight * uv.x + cameraUp * uv.y + cameraRay.direction);

    // Background color
    color = vec4(FOG_COLOR, 1.0);

    // Trace the ray's path
    MapInfo mapInfo = trace(cameraRay);

    // If the camera ray hit something
    if(mapInfo.hit < EPSILON) {
        //// Lighting

        // Setup the light source
        // Init light source at 5.0, 5.0, 5.0
        // Again, axes need to be flipped, note the -vec3
        Ray lightRay = initRay(-vec3(5.0, 5.0, 5.0), cameraRay.position);

        #if SHADOWS == 1
        // Trace shadows
        float shadow = softshadow(lightRay, SHADOW_SOFTNESS);
        #else
        float shadow = 1.0;
        #endif

        // Light stuff
        vec3 normal = calcNormal(cameraRay.position); // Calculate the normal of the position on the map
        float distanceFade =  LIGHT_INTENSITY / pow(distance(lightRay.origin, lightRay.target), 2.0); // Light fades by the inverse square of distance
        float diffuse = max(0.0, dot(-lightRay.direction, normal)) * shadow * distanceFade; // Multiply diffuse by shadow and distance fade
        float specular = pow(diffuse, 32.0); // Specular lighting factor

        vec3 hitColor = mapInfo.color; // Get color from map info

        hitColor *= diffuse + specular + AMBIENT_LIGHT; // Add lighting values

        #if FOG == 1
        hitColor = mix(hitColor, FOG_COLOR, clamp(1.0 - exp(-max(distance(cameraRay.origin, cameraRay.position)- FOG_DISTANCE, 1.0) * 0.1), 0.0, 1.0)); // Add fog
        #endif

        color = vec4(hitColor, 1.0); // Set output color
    }
}
