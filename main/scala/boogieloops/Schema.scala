package boogieloops.schema

import boogieloops.schema.primitives.*
import boogieloops.schema.complex.*
import boogieloops.schema.composition.*
import boogieloops.schema.references.*
import boogieloops.schema.modifiers.*

import boogieloops.schema.validation.{ValidationResult, ValidationContext}

/**
 * Core Schema trait - represents a JSON Schema with compile-time type information
 *
 * This is the foundation trait for all Schema types, providing JSON Schema 2020-12 compliance while maintaining compile-time type
 * safety through Scala 3's type system.
 */
trait Schema {
  // Core vocabulary keywords (JSON Schema 2020-12)
  def $schema: Option[String] = None
  def $id: Option[String] = None
  def $ref: Option[String] = None
  def $defs: Option[Map[String, Schema]] = None
  def $dynamicRef: Option[String] = None
  def $dynamicAnchor: Option[String] = None
  def $vocabulary: Option[Map[String, Boolean]] = None
  def $comment: Option[String] = None

  // Meta-data vocabulary keywords
  def title: Option[String] = None
  def description: Option[String] = None
  def default: Option[ujson.Value] = None
  def examples: Option[List[ujson.Value]] = None
  def readOnly: Option[Boolean] = None
  def writeOnly: Option[Boolean] = None
  def deprecated: Option[Boolean] = None

  // Generate JSON Schema representation
  def toJsonSchema: ujson.Value

  // Validation framework interface
  /**
   * Validate a ujson.Value against this schema using ValidationContext
   * This provides the unified validation framework interface
   */
  def validate(value: ujson.Value, context: ValidationContext): ValidationResult

  // Modifier methods for chaining
  def optional: Schema = OptionalSchema(this)
  def nullable: Schema = NullableSchema(this)
  def withDefault(value: ujson.Value): Schema = DefaultSchema(this, value)
  def withTitle(title: String): Schema = TitleSchema(this, title)
  def withDescription(desc: String): Schema = DescriptionSchema(this, desc)
  def withSchema(schema: String): Schema = SchemaModifier(this, schema)
  def withId(id: String): Schema = IdSchema(this, id)
  def withDefs(defs: (String, Schema)*): Schema = boogieloops.schema.modifiers.DefsSchema(Some(this), defs.toMap)
  def withExamples(examples: ujson.Value*): Schema = ExamplesSchema(this, examples.toList)
}

/**
 * Modifier wrapper for optional fields
 */
case class OptionalSchema[T <: Schema](underlying: T) extends Schema {
  override def toJsonSchema: ujson.Value = underlying.toJsonSchema

  override def validate(value: ujson.Value, context: ValidationContext): ValidationResult = {
    underlying.validate(value, context)
  }

  override def nullable: Schema = OptionalNullableSchema(underlying)
}

/**
 * Modifier wrapper for nullable fields
 */
case class NullableSchema[T <: Schema](underlying: T) extends Schema {
  override def toJsonSchema: ujson.Value = {
    val base = underlying.toJsonSchema
    base("type") = ujson.Arr(base("type"), ujson.Str("null"))
    base
  }

  override def validate(value: ujson.Value, context: ValidationContext): ValidationResult = {
    value match {
      case ujson.Null => ValidationResult.valid() // null is valid for nullable schemas
      case _ => underlying.validate(value, context)
    }
  }

  override def optional: Schema = OptionalNullableSchema(underlying)
}

/**
 * Modifier wrapper for optional AND nullable fields
 */
case class OptionalNullableSchema[T <: Schema](underlying: T) extends Schema {
  override def toJsonSchema: ujson.Value = {
    val base = underlying.toJsonSchema
    base("type") = ujson.Arr(base("type"), ujson.Str("null"))
    base
  }

  override def validate(value: ujson.Value, context: ValidationContext): ValidationResult = {
    value match {
      case ujson.Null => ValidationResult.valid() // null is valid for nullable schemas
      case _ => underlying.validate(value, context)
    }
  }
}

/**
 * Modifier wrapper for fields with default values
 */
case class DefaultSchema[T <: Schema](underlying: T, defaultValue: ujson.Value) extends Schema {
  override def toJsonSchema: ujson.Value = {
    val base = underlying.toJsonSchema
    base("default") = defaultValue
    base
  }

  override def validate(value: ujson.Value, context: ValidationContext): ValidationResult = {
    underlying.validate(value, context)
  }

  override def optional: Schema = OptionalSchema(this)
  override def nullable: Schema = NullableSchema(this)
}

/**
 * Companion object with factory methods
 */
object bl {
  // JSON Schema 2020-12 meta-schema URL
  val MetaSchemaUrl = "https://json-schema.org/draft/2020-12/schema"

  // Convenience factory methods
  def String(
      minLength: Option[Int] = None,
      maxLength: Option[Int] = None,
      pattern: Option[String] = None,
      format: Option[String] = None,
      const: Option[String] = None
  ): StringSchema = StringSchema(minLength, maxLength, pattern, format, const)

  def Number(
      minimum: Option[Double] = None,
      maximum: Option[Double] = None,
      exclusiveMinimum: Option[Double] = None,
      exclusiveMaximum: Option[Double] = None,
      multipleOf: Option[Double] = None
  ): NumberSchema = NumberSchema(minimum, maximum, exclusiveMinimum, exclusiveMaximum, multipleOf)

  def Integer(
      minimum: Option[Int] = None,
      maximum: Option[Int] = None,
      exclusiveMinimum: Option[Int] = None,
      exclusiveMaximum: Option[Int] = None,
      multipleOf: Option[Int] = None,
      const: Option[Int] = None
  ): IntegerSchema =
    IntegerSchema(minimum, maximum, exclusiveMinimum, exclusiveMaximum, multipleOf, const)

  def Boolean(const: Option[Boolean] = None): BooleanSchema = BooleanSchema(const)

  def Null(): NullSchema = NullSchema()

  def Array[T <: Schema](
      items: T,
      minItems: Option[Int] = None,
      maxItems: Option[Int] = None,
      uniqueItems: Option[Boolean] = None,
      prefixItems: Option[List[Schema]] = None
  ): ArraySchema[T] = ArraySchema(items, minItems, maxItems, uniqueItems, prefixItems)

  def Object(
      properties: (String, Schema)*
  ): ObjectSchema = ObjectSchema(properties.toMap)

  def Object(
      properties: Map[String, Schema],
      required: Set[String] = Set.empty,
      minProperties: Option[Int] = None,
      maxProperties: Option[Int] = None,
      additionalProperties: Option[Boolean] = None,
      additionalPropertiesSchema: Option[Schema] = None,
      patternProperties: Map[String, Schema] = Map.empty,
      propertyNames: Option[Schema] = None
  ): ObjectSchema = {
    ObjectSchema(
      properties,
      required,
      minProperties,
      maxProperties,
      additionalProperties,
      additionalPropertiesSchema,
      patternProperties,
      propertyNames
    )
  }

  // Composition keywords
  def AnyOf(schemas: Schema*): AnyOfSchema = AnyOfSchema(schemas.toList)
  def OneOf(schemas: Schema*): OneOfSchema = OneOfSchema(schemas.toList)
  def AllOf(schemas: Schema*): AllOfSchema = AllOfSchema(schemas.toList)
  def Not(schema: Schema): NotSchema = NotSchema(schema)

  // Conditional schemas
  def If(condition: Schema, thenSchema: Schema, elseSchema: Schema): IfThenElseSchema =
    IfThenElseSchema(condition, Some(thenSchema), Some(elseSchema))
  def If(condition: Schema, thenSchema: Schema): IfThenElseSchema =
    IfThenElseSchema(condition, Some(thenSchema), None)

  // References
  def Ref(ref: String): RefSchema = RefSchema(ref)
  def DynamicRef(ref: String): DynamicRefSchema = DynamicRefSchema(ref)

  // Definitions
  def Defs(defs: (String, Schema)*): DefsSchema = modifiers.DefsSchema(None, defs.toMap)
  def Defs(defs: Map[String, Schema]): DefsSchema = modifiers.DefsSchema(None, defs)

  // Enum factory methods
  def StringEnum(values: String*): EnumSchema = EnumSchema.fromStrings(values*)
  def StringEnum(values: List[String]): EnumSchema = EnumSchema.fromStrings(values)
  def MixedEnum(values: ujson.Value*): EnumSchema = EnumSchema.fromValues(values*)
  def MixedEnum(values: List[ujson.Value]): EnumSchema = EnumSchema.fromValues(values)
}
