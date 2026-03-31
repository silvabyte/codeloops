package boogieloops.schema.derivation

/**
 * Schema annotation classes for JSON Schema 2020-12 metadata
 *
 * These annotations can be applied to case classes and fields to provide
 * rich metadata that gets included in the generated JSON schema.
 */
object SchemaAnnotations {

  /**
   * Annotation for schema title
   */
  class title(val value: String) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for schema description
   */
  class description(val value: String) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for string format validation
   */
  class format(val value: String) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for minimum length (strings/arrays)
   */
  class minLength(val value: Int) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for maximum length (strings/arrays)
   */
  class maxLength(val value: Int) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for minimum value (numbers)
   */
  class minimum(val value: Double) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for maximum value (numbers)
   */
  class maximum(val value: Double) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for regex pattern (strings)
   */
  class pattern(val value: String) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for minimum number of items (arrays)
   */
  class minItems(val value: Int) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for maximum number of items (arrays)
   */
  class maxItems(val value: Int) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for unique items constraint (arrays)
   */
  class uniqueItems(val value: Boolean = true) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for multiple of constraint (numbers)
   */
  class multipleOf(val value: Double) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for exclusive minimum (numbers)
   */
  class exclusiveMinimum(val value: Double) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for exclusive maximum (numbers)
   */
  class exclusiveMaximum(val value: Double) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for enumeration values - supports strings, numbers, and booleans
   */
  class enumValues(val values: (String | Int | Boolean | Double | Null)*)
      extends scala.annotation.StaticAnnotation

  /**
   * Annotation for constant value - supports strings, numbers, and booleans
   */
  class const(val value: String | Int | Boolean | Double | Null)
      extends scala.annotation.StaticAnnotation

  /**
   * Annotation for default values - supports all types via union type
   */
  class default(val value: String | Int | Boolean | Double)
      extends scala.annotation.StaticAnnotation

  /**
   * Annotation for example values (as JSON strings)
   */
  class examples(val values: String*) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for read-only fields
   */
  class readOnly(val value: Boolean = true) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for write-only fields
   */
  class writeOnly(val value: Boolean = true) extends scala.annotation.StaticAnnotation

  /**
   * Annotation for deprecated fields/schemas
   */
  class deprecated(val value: Boolean = true) extends scala.annotation.StaticAnnotation
}
