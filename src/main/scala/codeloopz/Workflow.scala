package codeloopz

import scala.util.Try
import scala.concurrent.{Future, Await}
import scala.concurrent.duration.Duration
import scala.concurrent.ExecutionContext.Implicits.global

case class WorkflowError(message: String)
    extends Exception(message)

case class Workflow[A](run: WorkflowContext => Either[WorkflowError, A]):
  def map[B](f: A => B): Workflow[B] =
    Workflow(ctx => run(ctx).map(f))

  def flatMap[B](f: A => Workflow[B]): Workflow[B] =
    Workflow(ctx => run(ctx).flatMap(a => f(a).run(ctx)))

object Workflow:
  def pure[A](a: A): Workflow[A] =
    Workflow(_ => Right(a))

  def fail[A](err: WorkflowError): Workflow[A] =
    Workflow(_ => Left(err))

  def fail[A](message: String): Workflow[A] =
    Workflow(_ => Left(WorkflowError(message)))

  def agent(
      prompt: Prompt,
      inputs: Map[String, String] = Map.empty
  ): Workflow[String] = {
    Workflow { ctx =>
      for
        rendered <- prompt.render(inputs)
        _ = ctx.logger.info(s"Running agent with prompt (${rendered.take(80)}...)")
        output <- ctx.agent.run(rendered, ctx.workDir, ctx.model)
        result <- output.exitCode match
          case 0    => Right(output.stdout)
          case code =>
            Left(WorkflowError(
              s"Agent exited with code $code: ${output.stderr}"
            ))
      yield result
    }
  }

  def parallel[A](workflows: Seq[Workflow[A]]): Workflow[Seq[A]] = {
    Workflow { ctx =>
      try {
        val futures = workflows.map { w =>
          Future(w.run(ctx)).flatMap {
            case Right(a)  => Future.successful(a)
            case Left(err) => Future.failed(err)
          }
        }
        Right(Await.result(Future.sequence(futures), Duration.Inf))
      } catch {
        case we: WorkflowError => Left(we)
        case e:  Exception     => Left(WorkflowError(e.getMessage))
      }
    }
  }

  def loop(
      step:          Option[String] => Workflow[String],
      maxIterations: Int = 20
  ): Workflow[String] = {
    Workflow { ctx =>
      def iterate(
          iteration:        Int,
          previousFeedback: Option[String]
      ): Either[WorkflowError, String] = {
        if iteration >= maxIterations then {
          Left(WorkflowError(
            s"Loop exceeded max iterations ($maxIterations)"
          ))
        } else {
          step(previousFeedback).run(ctx).flatMap { result =>
            parseDoneSignal(result) match
              case Some(finalResult) =>
                ctx.logger.info(
                  s"Loop completed after ${iteration + 1} iteration(s)"
                )
                Right(finalResult)
              case None =>
                ctx.logger.info(
                  s"Loop iteration ${iteration + 1}, continuing..."
                )
                iterate(iteration + 1, Some(result))
          }
        }
      }

      iterate(1, None)
    }
  }

  private def parseDoneSignal(response: String): Option[String] = {
    Try {
      val json = ujson.read(response)
      val done = json("done").bool
      if done then Some(json("review").str)
      else None
    }.getOrElse(None)
  }
