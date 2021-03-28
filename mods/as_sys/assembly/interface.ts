export declare function just_pressed(ref: externref, input: i32): boolean;
export declare function just_released(ref: externref, input: i32): boolean;


// Size of value at Vec3.ptr
export declare const VEC3_SIZE: usize;
/**
 * Wrapper class to expose Vec3 fucntionality that is imported from host
 */
@unmanaged
export class Vec3 {
    
    ptr: usize;
    // pointer to the underlying Vec3 in mem
    constructor(
        ptr: usize
    ) {
        this.ptr = ptr;
    }

    static unit_z(): Vec3 {
        let ptr = heap.alloc(VEC3_SIZE);
        _unit_z(ptr);
        return new Vec3(ptr);
    }

    normalize(): Vec3 {
        _normalize(this.ptr);
        return this;
    }
}

// Size of value at Quat.ptr
export declare const QUAT_SIZE: usize;
/**
 * Wrapper class to expose Quat fucntionality that is imported from host
 */
@unmanaged
export class Quat {
    ptr: usize;

    // pointer to the underlying Quat in mem
    constructor(
        ptr: usize
    ) {
        this.ptr = ptr
    }

    getX(): f32 {
        return load<f32>(this.ptr, 0);
    }

    getY(): f32 {
        return load<f32>(this.ptr, 4);
    }

    getZ(): f32 {
        return load<f32>(this.ptr, 8);
    }

    getW(): f32 {
        return load<f32>(this.ptr, 12);
    }

    mul_vec3(v: Vec3): Vec3 {
       let out_ptr = heap.alloc(VEC3_SIZE);
       _mul_vec3(this.ptr, v.ptr, out_ptr);
       return new Vec3(out_ptr);
    }
}

export declare function _unit_z(ptr: usize): void; 
export declare function _mul_vec3(rot: usize, v: usize, out: usize): void;
export declare function _normalize(out: usize): void;