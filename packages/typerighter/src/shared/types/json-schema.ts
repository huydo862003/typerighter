// JSON Schema types and builders used for the component contract
// Covers the subset of JSON Schema that typedown component props can express:
// primitives, arrays, objects (dict and product), and optional (nullable) variants

// { type: "array", items: <schema> }
export interface ArraySchema {
  type: 'array';
  items: JsonSchema;
}

// { type: "object", additionalProperties: <value schema> }
export interface DictSchema {
  type: 'object';
  additionalProperties: JsonSchema;
}

// A JSON Schema object node
export type JsonSchema =
  | PrimitiveSchema
  | OptionalSchema
  | ArraySchema
  | DictSchema
  | ProductSchema;

// Nullable type: { type: ["T", "null"] } or with "undefined" when TypeScript optional surfaces it
// JSON Schema uses "null"; TypeScript optional also surfaces "undefined"
export interface OptionalSchema {
  type: [string, 'null' | 'undefined'] | [string, 'null', 'undefined'] | [string, 'undefined', 'null'];
}

export interface PrimitiveSchema {
  type: 'string' | 'number' | 'boolean';
}

// { type: "object", properties: {...}, required?: [...], additionalProperties: false }
export interface ProductSchema {
  type: 'object';
  properties: Record<string, JsonSchema>;
  required?: string[];
  additionalProperties: false;
}

export function isOptional (schema: JsonSchema): boolean {
  return Array.isArray(schema.type);
}

// Wrap any schema in a nullable (optional) variant
// No-op if already optional (type is already an array)
export function makeOptional (schema: JsonSchema): JsonSchema {
  const baseType = schema.type;

  if (typeof baseType === 'string') {
    return {
      ...schema,
      type: [
        baseType,
        'null',
      ],
    };
  }

  return schema;
}
