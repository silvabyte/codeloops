package boogieloops.schema.examples

import boogieloops.schema.*

/**
 * Basic usage examples of the BoogieLoops library
 */
object BasicUsage {

  def main(args: Array[String]): Unit = {
    println("🎉 BoogieLoops Library - JSON Schema generation using upickle for Scala 3!")
    println("=" * 50)

    // Basic primitive schemas
    println("\n1. Primitive Schemas:")

    val stringSchema = bl.String(
      minLength = Some(1),
      maxLength = Some(100),
      pattern = Some("^[a-zA-Z]+$")
    )
    println(s"String Schema: ${stringSchema.toJsonSchema}")

    val numberSchema = bl.Number(
      minimum = Some(0.0),
      maximum = Some(100.0),
      multipleOf = Some(0.1)
    )
    println(s"Number Schema: ${numberSchema.toJsonSchema}")

    val integerSchema = bl.Integer(
      minimum = Some(1),
      maximum = Some(1000)
    )
    println(s"Integer Schema: ${integerSchema.toJsonSchema}")

    // Complex schemas
    println("\n2. Complex Schemas:")

    val userSchema = bl.Object(
      "id" -> bl.String(),
      "name" -> bl.String(minLength = Some(1)),
      "email" -> bl.String(format = Some("email")),
      "age" -> bl.Integer(minimum = Some(0)).optional
    )
    println(s"User Schema: ${userSchema.toJsonSchema}")

    val arraySchema = bl.Array(
      bl.String(),
      minItems = Some(1),
      maxItems = Some(10),
      uniqueItems = Some(true)
    )
    println(s"Array Schema: ${arraySchema.toJsonSchema}")

    // Null handling with modifier pattern
    println("\n3. Null Handling:")

    val nullableString = bl.String().nullable
    println(s"Nullable String: ${nullableString.toJsonSchema}")

    val optionalString = bl.String().optional
    println(s"Optional String: ${optionalString.toJsonSchema}")

    val optionalNullableString = bl.String().optional.nullable
    println(s"Optional Nullable String: ${optionalNullableString.toJsonSchema}")

    // JSON Schema 2020-12 composition keywords
    println("\n4. Composition Keywords:")

    val anyOfSchema = bl.AnyOf(
      bl.String(),
      bl.Number()
    )
    println(s"AnyOf Schema: ${anyOfSchema.toJsonSchema}")

    val oneOfSchema = bl.OneOf(
      bl.String(),
      bl.Integer()
    )
    println(s"OneOf Schema: ${oneOfSchema.toJsonSchema}")

    val allOfSchema = bl.AllOf(
      bl.Object("name" -> bl.String()),
      bl.Object("age" -> bl.Integer())
    )
    println(s"AllOf Schema: ${allOfSchema.toJsonSchema}")

    val notSchema = bl.Not(bl.String())
    println(s"Not Schema: ${notSchema.toJsonSchema}")

    // Conditional schemas (if/then/else)
    println("\n5. Conditional Schemas:")

    val conditionalSchema = bl.If(
      condition = bl.Object("type" -> bl.String()),
      thenSchema = bl.Object("name" -> bl.String()),
      elseSchema = bl.Object("id" -> bl.Integer())
    )
    println(s"Conditional Schema: ${conditionalSchema.toJsonSchema}")

    // References
    println("\n6. References:")

    val refSchema = bl.Ref("#/$defs/User")
    println(s"Reference Schema: ${refSchema.toJsonSchema}")

    val dynamicRefSchema = bl.DynamicRef("#user")
    println(s"Dynamic Reference Schema: ${dynamicRefSchema.toJsonSchema}")

    // Complex nested example
    println("\n7. Complex Nested Example:")

    val productSchema = bl.Object(
      "id" -> bl.String(),
      "name" -> bl.String(minLength = Some(1)),
      "price" -> bl.Number(minimum = Some(0)),
      "category" -> bl.OneOf(
        bl.String(),
        bl.Object("id" -> bl.String(), "name" -> bl.String())
      ),
      "tags" -> bl.Array(bl.String()).optional,
      "metadata" -> bl.Object().optional.nullable
    )

    println(s"Product Schema: ${productSchema.toJsonSchema}")

    // Demonstrate JSON Schema 2020-12 compliance
    println("\n8. JSON Schema 2020-12 Compliance:")

    val compliantSchema = bl
      .Object(
        "version" -> bl.String()
      )
      .withSchema(bl.MetaSchemaUrl)
      .withId("https://example.com/schemas/product")
      .withTitle("Product Schema")
      .withDescription("A schema for product objects")

    println(s"Compliant Schema: ${compliantSchema.toJsonSchema}")

    println("\n🎯 All examples completed successfully!")
    println("BoogieLoops provides full JSON Schema 2020-12 compliance with TypeBox-like ergonomics!")
  }
}
