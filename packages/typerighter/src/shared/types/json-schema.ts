// JSON Schema subset for component prop contracts

// { anyOf: [<schema>, ...] }
export interface AnyOfSchema {
  anyOf: JsonSchema[];
}

// { type: "array", items: <schema> }
export interface ArraySchema {
  type: 'array';
  items: JsonSchema;
}

// { type: "object", additionalProperties: <schema> }
export interface DictSchema {
  type: 'object';
  additionalProperties: JsonSchema;
}

// { enum: ["info", "warning", null] }
export interface EnumSchema {
  enum: (string | number | boolean | null)[];
}

export type JsonSchema =
  | PrimitiveSchema
  | MultiTypeSchema
  | EnumSchema
  | ArraySchema
  | ProductSchema
  | DictSchema
  | OneOfSchema
  | AnyOfSchema;

// { type: ["string", "null"] } or { type: ["string", "number"] }
export interface MultiTypeSchema {
  type: string[];
}

// { oneOf: [<schema>, ...] }
export interface OneOfSchema {
  oneOf: JsonSchema[];
}

// { type: "string" }
export interface PrimitiveSchema {
  type: 'string' | 'number' | 'boolean';
}

// { type: "object", properties: {...}, required: [...], additionalProperties: false }
export interface ProductSchema {
  type: 'object';
  properties: Record<string, JsonSchema>;
  required?: string[];
  additionalProperties: false;
}

export function isOptional (schema: JsonSchema): boolean {
  if ('enum' in schema) return schema.enum.includes(null);
  if ('type' in schema && Array.isArray(schema.type)) return schema.type.includes('null');

  return false;
}

export function makeOptional (schema: JsonSchema): JsonSchema {
  if ('enum' in schema) {
    if (schema.enum.includes(null)) return schema;

    return {
      enum: [
        ...schema.enum,
        null,
      ],
    };
  }

  if ('type' in schema) {
    if (typeof schema.type === 'string') {
      return {
        ...schema,
        type: [
          schema.type,
          'null',
        ],
      };
    }

    if (Array.isArray(schema.type) && !schema.type.includes('null')) {
      return {
        ...schema,
        type: [
          ...schema.type,
          'null',
        ],
      };
    }
  }

  return schema;
}
