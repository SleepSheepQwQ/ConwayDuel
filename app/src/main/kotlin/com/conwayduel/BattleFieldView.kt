package com.conwayduel

import android.content.Context
import android.graphics.*
import android.view.View
import java.util.*
import kotlin.concurrent.timerTask
import kotlin.math.*

class BattleFieldView(ctx: Context) : View(ctx) {

    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val planes = mutableListOf<Plane>()
    private val bullets = mutableListOf<Bullet>()
    private var lastTime = System.currentTimeMillis()
    private var borderAlpha = 0.3f
    private var isGameOver = false
    private var winnerTeam: Team? = null

    // 边界闪烁定时器
    private val borderTimer = Timer()

    init {
        setLayerType(LAYER_TYPE_HARDWARE, null)
        // 初始化三队飞机
        planes.add(Plane(300f, 400f, team = Team.RED))
        planes.add(Plane(900f, 400f, team = Team.GREEN))
        planes.add(Plane(600f, 800f, team = Team.BLUE))

        // 边界若隐若现效果
        borderTimer.schedule(timerTask {
            borderAlpha = if (borderAlpha < 0.5f) 0.8f else 0.3f
            postInvalidate()
        }, 0, 600)
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val fieldWidth = width.toFloat()
        val fieldHeight = height.toFloat()
        val borderMargin = 50f

        // 1. 绘制黑色背景
        canvas.drawColor(Color.BLACK)

        // 2. 绘制藏蓝色若隐若现边界（触边即死）
        paint.color = Color.argb((borderAlpha * 255).toInt(), 0, 20, 139)
        paint.style = Paint.Style.STROKE
        paint.strokeWidth = 8f
        canvas.drawRect(
            borderMargin,
            borderMargin,
            fieldWidth - borderMargin,
            fieldHeight - borderMargin,
            paint
        )

        // 游戏结束，绘制获胜信息
        if (isGameOver) {
            paint.color = winnerTeam?.color ?: Color.WHITE
            paint.textSize = 120f
            paint.textAlign = Paint.Align.CENTER
            paint.style = Paint.Style.FILL
            canvas.drawText(
                "${winnerTeam?.name} WIN!",
                fieldWidth / 2,
                fieldHeight / 2,
                paint
            )
            postInvalidate()
            return
        }

        // 计算帧间隔时间
        val now = System.currentTimeMillis()
        val deltaTime = (now - lastTime) / 1000f
        lastTime = now

        // 3. 更新所有飞机AI
        planes.forEach { plane ->
            AiLogic.updatePlaneAI(
                plane,
                planes,
                bullets,
                fieldWidth,
                fieldHeight,
                deltaTime
            )

            // 触边死亡判定
            if (plane.x < borderMargin || plane.x > fieldWidth - borderMargin
                || plane.y < borderMargin || plane.y > fieldHeight - borderMargin
            ) {
                plane.alive = false
            }

            // 尝试射击
            AiLogic.tryShoot(plane, planes)?.let {
                bullets.add(it)
            }
        }

        // 4. 更新子弹位置
        bullets.forEach { bullet ->
            bullet.x += cos(bullet.dir) * bullet.speed * deltaTime
            bullet.y += sin(bullet.dir) * bullet.speed * deltaTime
        }

        // 5. 子弹碰撞判定（扣血）
        val hitBullets = mutableListOf<Bullet>()
        planes.forEach { plane ->
            if (!plane.alive) return@forEach
            bullets.forEach { bullet ->
                if (bullet.team == plane.team) return@forEach
                val dist = hypot(bullet.x - plane.x, bullet.y - plane.y)
                // 碰撞半径30px
                if (dist < 30f) {
                    hitBullets.add(bullet)
                    plane.hp -= bullet.damage
                    if (plane.hp <= 0) {
                        plane.alive = false
                    }
                }
            }
        }

        // 移除命中的子弹
        bullets.removeAll(hitBullets)
        // 移除出界的子弹
        bullets.removeAll {
            it.x < 0 || it.x > fieldWidth || it.y < 0 || it.y > fieldHeight
        }
        // 移除死亡飞机
        planes.removeAll { !it.alive }

        // 6. 胜负判定
        val aliveTeams = planes.map { it.team }.distinct()
        if (aliveTeams.size == 1) {
            isGameOver = true
            winnerTeam = aliveTeams.first()
        }

        // 7. 绘制所有子弹
        bullets.forEach { bullet ->
            paint.color = bullet.team.color
            paint.style = Paint.Style.FILL
            canvas.drawCircle(bullet.x, bullet.y, 6f, paint)
        }

        // 8. 绘制所有飞机 + 外围虚线圆环
        planes.forEach { plane ->
            paint.color = plane.team.color
            paint.style = Paint.Style.FILL

            // 绘制纸飞机
            canvas.save()
            canvas.translate(plane.x, plane.y)
            canvas.rotate(Math.toDegrees(plane.dir.toDouble()).toFloat())
            val planePath = Path()
            planePath.moveTo(30f, 0f)
            planePath.lineTo(-15f, -20f)
            planePath.lineTo(-10f, 0f)
            planePath.lineTo(-15f, 20f)
            planePath.close()
            canvas.drawPath(planePath, paint)

            // 绘制外围间隔虚线圆环
            paint.style = Paint.Style.STROKE
            paint.strokeWidth = 2f
            paint.pathEffect = DashPathEffect(floatArrayOf(10f, 8f), 0f)
            canvas.drawCircle(0f, 0f, 50f, paint)
            paint.pathEffect = null
            canvas.restore()

            // 绘制血量条
            val hpBarWidth = 60f
            val hpBarHeight = 6f
            val hpPercent = plane.hp / 5f
            // 背景
            paint.color = Color.DKGRAY
            canvas.drawRect(
                plane.x - hpBarWidth/2,
                plane.y - 60f,
                plane.x + hpBarWidth/2,
                plane.y - 60f + hpBarHeight,
                paint
            )
            // 血量
            paint.color = plane.team.color
            canvas.drawRect(
                plane.x - hpBarWidth/2,
                plane.y - 60f,
                plane.x - hpBarWidth/2 + hpBarWidth * hpPercent,
                plane.y - 60f + hpBarHeight,
                paint
            )
        }

        // 持续刷新
        invalidate()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        borderTimer.cancel()
    }
}
