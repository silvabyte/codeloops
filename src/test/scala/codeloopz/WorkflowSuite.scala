package codeloopz

class WorkflowSuite extends munit.FunSuite:

  val mockCtx: WorkflowContext = WorkflowContext(
    workDir = os.pwd,
    agent = AgentRunner.mock(prompt => s"response to: ${prompt.take(50)}"),
    logger = Logger.silent
  )

  test("pure lifts a value into Right") {
    assertEquals(Workflow.pure(42).run(mockCtx), Right(42))
  }

  test("fail lifts into Left") {
    val err = WorkflowError("boom")
    assertEquals(Workflow.fail(err).run(mockCtx), Left(err))
  }

  test("map transforms the result") {
    val w = Workflow.pure(10).map(_ * 2)
    assertEquals(w.run(mockCtx), Right(20))
  }

  test("flatMap chains workflows") {
    val w = for
      a <- Workflow.pure(3)
      b <- Workflow.pure(a + 7)
    yield b
    assertEquals(w.run(mockCtx), Right(10))
  }

  test("flatMap short-circuits on failure") {
    var secondRan = false
    val w = for
      _ <- Workflow.fail(WorkflowError("early fail"))
      _ <- Workflow { _ => secondRan = true; Right(()) }
    yield ()
    assert(w.run(mockCtx).isLeft)
    assert(!secondRan)
  }

  test("agent runs prompt through mock runner") {
    val w = Workflow.agent("hello world")
    val result = w.run(mockCtx)
    assert(result.isRight)
    assert(result.exists(_.contains("response to: hello world")))
  }

  test("agent substitutes template variables") {
    val w = Workflow.agent("review {{file}}", Map("file" -> "Main.scala"))
    val ctx = mockCtx.copy(
      agent = AgentRunner.mock(prompt => prompt)
    )
    val result = w.run(ctx)
    assertEquals(result, Right("review Main.scala"))
  }

  test("agent propagates failure on non-zero exit code") {
    val failAgent = new AgentRunner:
      def run(
          prompt: String,
          workDir: os.Path,
          model: Option[String]
      ): Either[WorkflowError, AgentOutput] =
        Right(AgentOutput("", "bad stuff", exitCode = 1))

    val ctx = mockCtx.copy(agent = failAgent)
    val result = Workflow.agent("test").run(ctx)
    assert(result.isLeft)
  }

  test("parallel collects results from all workflows") {
    var counter = 0
    val workflows = (1 to 3).map { i =>
      Workflow[Int] { _ =>
        synchronized { counter += 1 }
        Right(i * 10)
      }
    }
    val result = Workflow.parallel(workflows).run(mockCtx)
    assertEquals(result, Right(Seq(10, 20, 30)))
    assertEquals(counter, 3)
  }

  test("parallel fails if any workflow fails") {
    val workflows: Seq[Workflow[Int]] = Seq(
      Workflow.pure(1),
      Workflow.fail(WorkflowError("parallel boom")),
      Workflow.pure(3)
    )
    val result = Workflow.parallel(workflows).run(mockCtx)
    assert(result.isLeft)
  }

  test("loop terminates on done signal") {
    var callCount = 0
    val step = (_: Option[String]) =>
      Workflow[String] { _ =>
        callCount += 1
        val response =
          if callCount >= 2 then
            """{"done": true, "review": "final review"}"""
          else """{"done": false, "review": "still refining"}"""
        Right(response)
      }
    val result = Workflow.loop(step, maxIterations = 5).run(mockCtx)
    assertEquals(result, Right("final review"))
    assertEquals(callCount, 2)
  }

  test("loop passes prior feedback to subsequent iterations") {
    var receivedFeedback: List[Option[String]] = Nil
    val step = (prior: Option[String]) =>
      Workflow[String] { _ =>
        receivedFeedback = receivedFeedback :+ prior
        if receivedFeedback.size >= 3 then
          Right("""{"done": true, "review": "done"}""")
        else
          Right("""{"done": false, "review": "needs work"}""")
      }
    Workflow.loop(step, maxIterations = 5).run(mockCtx)
    assertEquals(receivedFeedback(0), None)
    assertEquals(
      receivedFeedback(1),
      Some("""{"done": false, "review": "needs work"}""")
    )
  }

  test("loop fails on max iterations exceeded") {
    val step = (_: Option[String]) =>
      Workflow[String] { _ =>
        Right("""{"done": false, "review": "not yet"}""")
      }
    val result = Workflow.loop(step, maxIterations = 3).run(mockCtx)
    assert(result.isLeft)
    assert(result.left.toOption.get.message.contains("max iterations"))
  }

  test("Prompt.Inline resolves directly") {
    assertEquals(Prompt.Inline("hello").resolve, Right("hello"))
  }

  test("Prompt.FromFile fails on missing file") {
    val p = Prompt.FromFile(os.pwd / "nonexistent-prompt.md")
    assert(p.resolve.isLeft)
  }

  test("Prompt.render substitutes placeholders") {
    val p = Prompt.Inline("Hello {{name}}, you have {{count}} items")
    val result = p.render(Map("name" -> "Alice", "count" -> "5"))
    assertEquals(result, Right("Hello Alice, you have 5 items"))
  }

  test("for-comprehension composes full workflow") {
    val ctx = mockCtx.copy(
      agent = AgentRunner.mock { prompt =>
        if prompt.contains("diff") then "mock diff content"
        else if prompt.contains("review") then
          """{"done": true, "review": "looks good"}"""
        else s"processed: $prompt"
      }
    )

    val w = for
      diff   <- Workflow.agent("get the diff")
      review <- Workflow.agent("review {{diff}}", Map("diff" -> diff))
    yield review

    val result = w.run(ctx)
    assert(result.isRight)
  }

  test("AgentType.ClaudeCode builds correct args") {
    val args = AgentType.ClaudeCode.buildArgs("test prompt", Some("sonnet"))
    assertEquals(
      args,
      Seq(
        "claude",
        "--print",
        "--dangerously-skip-permissions",
        "--model",
        "sonnet",
        "--",
        "test prompt"
      )
    )
  }

  test("AgentType.ClaudeCode builds args without model") {
    val args = AgentType.ClaudeCode.buildArgs("test prompt", None)
    assertEquals(
      args,
      Seq(
        "claude",
        "--print",
        "--dangerously-skip-permissions",
        "--",
        "test prompt"
      )
    )
  }

  test("AgentType.OpenCode builds correct args") {
    val args = AgentType.OpenCode.buildArgs("test prompt", Some("gpt-4"))
    assertEquals(
      args,
      Seq("opencode", "run", "--model", "gpt-4", "--prompt", "test prompt")
    )
  }

  test("AgentType.Cursor builds correct args with default model") {
    val args = AgentType.Cursor.buildArgs("test prompt", None)
    assertEquals(
      args,
      Seq(
        "cursor-agent",
        "-p",
        "test prompt",
        "--output-format",
        "text",
        "--model",
        "opus-4.5-thinking"
      )
    )
  }

  test("AgentType.fromString parses known agents") {
    assertEquals(AgentType.fromString("claude"), Some(AgentType.ClaudeCode))
    assertEquals(
      AgentType.fromString("claude-code"),
      Some(AgentType.ClaudeCode)
    )
    assertEquals(AgentType.fromString("opencode"), Some(AgentType.OpenCode))
    assertEquals(AgentType.fromString("cursor"), Some(AgentType.Cursor))
  }

  test("AgentType.fromString returns None for unknown") {
    assertEquals(AgentType.fromString("unknown-agent"), None)
  }

  test("AsPrompt enum converts to Prompt for Workflow.agent") {
    enum TestStep(val prompt: Prompt) extends Prompt.AsPrompt:
      case Greet extends TestStep(Prompt.Inline("Hello {{name}}"))

    val ctx = mockCtx.copy(agent = AgentRunner.mock(identity))
    val result = Workflow.agent(TestStep.Greet, Map("name" -> "World")).run(ctx)
    assertEquals(result, Right("Hello World"))
  }

  test("AsPrompt enum values work in Seq[Prompt]") {
    enum TestStep(val prompt: Prompt) extends Prompt.AsPrompt:
      case A extends TestStep(Prompt.Inline("alpha"))
      case B extends TestStep(Prompt.Inline("beta"))

    val prompts: Seq[Prompt] = Seq(TestStep.A, TestStep.B)
    assertEquals(prompts.size, 2)
    assertEquals(prompts.head.resolve, Right("alpha"))
    assertEquals(prompts(1).resolve, Right("beta"))
  }
