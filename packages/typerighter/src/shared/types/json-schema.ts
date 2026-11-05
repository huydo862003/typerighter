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

// { const: "auto" }
export interface ConstSchema {
  const: string | number | boolean | null;
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
  | ConstSchema
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
  if ('oneOf' in schema) return schema.oneOf.some((member) => 'const' in member && member.const === null);
  if ('anyOf' in schema) return schema.anyOf.some((member) => 'const' in member && member.const === null);

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

  if ('oneOf' in schema) {
    const hasNull = schema.oneOf.some((member) => 'const' in member && member.const === null);

    if (hasNull) return schema;

    return {
      oneOf: [
        ...schema.oneOf,
        {
          const: null,
        } satisfies ConstSchema,
      ],
    };
  }

  if ('anyOf' in schema) {
    const hasNull = schema.anyOf.some((member) => 'const' in member && member.const === null);

    if (hasNull) return schema;

    return {
      anyOf: [
        ...schema.anyOf,
        {
          const: null,
        } satisfies ConstSchema,
      ],
    };
  }

  return schema;
}
