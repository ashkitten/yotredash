#version 130

//// Output variables
out vec4 color;

//// Input variables
uniform float time;
uniform vec2 resolution;

//// Definitions

#define CAMERA_SPEED 0.5

#define MAX_DIST 50.0
#define EPSILON 0.0001

#define SHADOWS
#define SHADOW_THRESHOLD 0.01
#define SOFT_SHADOWS
#define SHADOW_HARDNESS 50.0

#define LIGHT_INTENSITY 25.0
#define AMBIENT_LIGHT 0.05

#define MAX_REFLECTIONS 5
#define REFLECTION_THRESHOLD 0.01

#define FOG
#define FOG_DISTANCE 25.0
#define FOG_COLOR vec3(0.2, 0.25, 0.3)

#define MATERIAL_LIGHTGRAY Material(vec3(0.9, 0.9, 0.9), 0.5, 8.0, 0.4)
#define MATERIAL_GREEN Material(vec3(0.258824, 0.956863, 0.607843), 0.8, 8.0, 0.7)
#define MATERIAL_BLUE Material(vec3(0.258824, 0.52549, 0.956863), 0.8, 4.0, 0.4)
#define MATERIAL_PURPLE Material(vec3(0.878431, 0.4, 1.0), 1.0, 8.0, 0.0)
#define MATERIAL_YELLOW Material(vec3(1.0, 0.843137, 0.0), 1.0, 8.0, 0.2)

//// Structs

struct Material {
    vec3 color;
    float diffuse;
    float specular;
    float reflectivity;
};

struct MapInfo {
    Material material;
    // Distance of the point we got info from to the map
    float hit;
};

struct Ray {
    // Origin of the ray
    vec3 origin;
    // Ray's direction
    vec3 direction;

    // Where the ray is aiming at
    vec3 target;

    // Current position along the ray
    float length;
    // Just so we don't have to calculate it multiple times ¯\_(ツ)_/¯
    vec3 position;
};

//// Ray initializer

Ray initRayToTarget(vec3 origin, vec3 target) {
    Ray ray;

    // Init ray values
    // Calculate ray direction
    ray.direction = normalize(target - origin);

    // Start a bit away from the origin so we don't hit whatever is at the start
    ray.origin = origin + ray.direction * EPSILON;

    ray.target = target;
    ray.position = origin;

    return ray;
}

Ray initRayToDirection(vec3 origin, vec3 direction) {
    Ray ray;

    // Init ray values
    ray.direction = direction;

    ray.origin = origin + ray.direction * EPSILON;
    ray.position = origin;

    return ray;
}

//// Distance functions

MapInfo plane(vec3 origin, Material material) {
    return MapInfo(material, -origin.y);
}

MapInfo sphere(vec3 origin, float rad, Material material) {
    return MapInfo(material, length(origin) - rad);
}

MapInfo box(vec3 origin, vec3 size, Material material) {
    return MapInfo(material, length(max(abs(origin) - size, 0.0)));
}

//// Distance operations

MapInfo opUnion(MapInfo o1, MapInfo o2) {
    // min(o1, o2)
    return (o1.hit < o2.hit) ? o1 : o2;
}

MapInfo opSubtract(MapInfo o1, MapInfo o2) {
    // max(-o1, o2)
    return (-o1.hit > o2.hit) ? MapInfo(o1.material, -o1.hit) : o2;
}

MapInfo opIntersect(MapInfo o1, MapInfo o2) {
    // max(o1, o2)
    return (o1.hit > o2.hit) ? o1 : o2;
}

//// The main mapping function

MapInfo map(vec3 origin) {
    // red plane at origin
    MapInfo mapInfo = plane(origin, MATERIAL_LIGHTGRAY);
    // blue sphere at 1.5, 1.0, 1.5 with radius 1.0
    mapInfo = opUnion(mapInfo, sphere(origin + vec3(1.5, 1.0, 1.5), 1.0, MATERIAL_BLUE));
    // green box at -1.5, 1.0, 1.5 with size 1.0 x 1.0 x 1.0
    mapInfo = opUnion(mapInfo, box(origin + vec3(-1.5, 1.0, 1.5), vec3(1.0), MATERIAL_GREEN));
    mapInfo = opUnion(mapInfo, opSubtract( // subtract sphere from box
        sphere(origin + vec3(-1.5, 1.0, -1.5), 1.2, MATERIAL_PURPLE),  // purple sphere at -1.5, 1.0, -1.5 with radius 1.2
        // purple box at -1.5, 1.0, -1.5 with size 1.0 x 1.0 x 1.0
        box(origin + vec3(-1.5, 1.0, -1.5), vec3(1.0), MATERIAL_PURPLE))
    );
    mapInfo = opUnion(mapInfo, opIntersect( // intersect sphere with box
        box(origin + vec3(1.5, 1.0, -1.5), vec3(1.0), MATERIAL_YELLOW), // yellow box at 1.5, 1.0, -1.5 with size 1.0 x 1.0 x 1.0
        // yellow sphere at 1.5, 1.0, -1.5 with radius 1.2
        sphere(origin + vec3(1.5, 1.0, -1.5), 1.2, MATERIAL_YELLOW))
    );
    return mapInfo;
}

//// Raymarching functions

vec3 calcNormal(vec3 position) {
    // Step around the point and see how far we are from the map at each position
    // and do some math to figure out what the normal is
    // I use a value of 0.02 because I find that's the best balance between accuracy and
    // avoiding those nasty rings on some objects
    vec2 eps = vec2(0.0, 0.005);
    return normalize(vec3(
        map(position + eps.yxx).hit - map(position - eps.yxx).hit,
        map(position + eps.xyx).hit - map(position - eps.xyx).hit,
        map(position + eps.xxy).hit - map(position - eps.xxy).hit
    ));
}

MapInfo trace(inout Ray ray) {
    while(distance(ray.position, ray.origin) < MAX_DIST) {
        // Get info about our position in relation to the map
        MapInfo mapInfo = map(ray.position);
        // If we hit something, return the info about our position on the map
        if(mapInfo.hit < EPSILON) return mapInfo;

        // Step forward along the ray, as far as our distance to the map
        ray.position += ray.direction * mapInfo.hit;
    }
    // Return fog if we didn't hit anything
    return MapInfo(Material(FOG_COLOR, 0.0, 0.0, 0.0), 1.0);
}

float softshadow(inout Ray ray, float softness) {
    // While we're not past the target, do the stuff
    // Subtract EPSILON * 2 so we don't get close enough to the original object to trigger the shadow
    float penumbra = 1.0;
    while(distance(ray.origin, ray.position) < distance(ray.origin, ray.target) - EPSILON * 2.0) {
        // Get info about our position in relation to the map
        MapInfo mapInfo = map(ray.position);
        // If we hit something, make the color black (shadow)
        if(mapInfo.hit < EPSILON) return 0.0;

        #ifdef SOFT_SHADOWS
            //TODO: fix soft shadows
            penumbra = min(penumbra, softness * mapInfo.hit / distance(ray.position, ray.target));
        #endif

        if(mapInfo.hit > SHADOW_THRESHOLD) ray.position += ray.direction * mapInfo.hit;
        // Move a bit closer to the target
        else ray.position += ray.direction * SHADOW_THRESHOLD;
    }
    // If we don't hit anything, the point is not in shadow so the shadow multiplier is 1.0
    return penumbra;
}

//// Main function

void main() {
    //// Setup the viewport
    // Screen coords go from -1.0 to 1.0
    vec2 uv = gl_FragCoord.xy / resolution.xy * 2.0 - 1.0;
    // Account for screen ratio
    uv.x *= resolution.x / resolution.y;

    //// Setup the camera
    // Init camera 5.0 up, rotating in a circle, looking at 0.0, 0.0, 0.0
    // Axes are flipped here I guess? Note the -vec3s
    Ray cameraRay = initRayToTarget(-vec3(
        sin(time * CAMERA_SPEED) * 5.0,
        5.0,
        cos(time * CAMERA_SPEED) * 5.0), -vec3(0.0, 0.0, 0.0)
    );
    // Convert to screen coords
    // Global up direction
    vec3 globalUp = vec3(0.0, 1.0, 0.0);
    // Right direction in relation to the camera
    vec3 cameraRight = normalize(cross(globalUp, cameraRay.origin));
    // Up direction in relation to the camera
    vec3 cameraUp = cross(cameraRay.direction, cameraRight);
    // Set the camera ray direction to point from the screen coordinate to our target
    cameraRay.direction = normalize(cameraRight * uv.x + cameraUp * uv.y + cameraRay.direction);

    vec4 reflections[MAX_REFLECTIONS + 1];

    // Background color
    float reflectivity = 1.0;
    for(int i = 0; i < MAX_REFLECTIONS + 1; i++) {
        // Trace the ray's path
        MapInfo mapInfo = trace(cameraRay);

        // If the camera ray hit something
        if(mapInfo.hit < EPSILON) {
            //// Lighting

            // Setup the light source
            // Init light source at 5.0, 5.0, 5.0
            // Again, axes need to be flipped, note the -vec3
            Ray lightRay = initRayToTarget(-vec3(5.0, 5.0, 5.0), cameraRay.position);

            #ifdef SHADOWS
                // Trace shadows
                float shadow = softshadow(lightRay, SHADOW_HARDNESS);
            #else
                float shadow = 1.0;
            #endif

            // Light stuff
            // Calculate the normal of the position on the map
            vec3 normal = calcNormal(cameraRay.position);
            // Light fades by the inverse square of distance
            float distanceFade =  LIGHT_INTENSITY / pow(distance(lightRay.origin, lightRay.target), 2.0);
            // Multiply diffuse by shadow and distance fade
            float diffuse = max(0.0, dot(-lightRay.direction, normal))
                * mapInfo.material.diffuse * shadow * distanceFade;
            // Specular lighting factor
            float specular = pow(diffuse, mapInfo.material.specular);

            // Get color from map info
            vec3 hitColor = mapInfo.material.color;

            // Add lighting values
            hitColor *= diffuse + specular + AMBIENT_LIGHT;

            #ifdef FOG
                // Add fog
                float fogAmount = 1.0 - exp(-max(distance(cameraRay.origin, cameraRay.position) - FOG_DISTANCE, 0.0) * 0.15);
                hitColor = mix(hitColor, FOG_COLOR, fogAmount);
            #endif

            reflections[i] = vec4(hitColor, mapInfo.material.reflectivity);

            // Set the camera ray to the reflection off the surface, and repeat
            cameraRay = initRayToDirection(
                cameraRay.position + normalize(reflect(cameraRay.direction, normal)) * EPSILON,
                normalize(reflect(cameraRay.direction, normal))
            );
        } else {
            reflections[i] = vec4(mapInfo.material.color, 0.0);
            break;
        }
    }

    color = vec4(reflections[MAX_REFLECTIONS - 1].rgb, 1.0);

    for(int i = MAX_REFLECTIONS - 2; i >= 0; i--) {
        color = mix(vec4(reflections[i].rgb, 1.0), color, reflections[i].a);
    }

    // Gamma correction
    color = vec4(pow(clamp(color.xyz, 0.0, 1.0), vec3(0.4545)), 1.0);
}
