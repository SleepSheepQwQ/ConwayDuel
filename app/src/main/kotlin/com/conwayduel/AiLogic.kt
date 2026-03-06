package com.conwayduel

import android.graphics.PointF
import kotlin.math.*
import kotlin.random.Random

object AiLogic {

    fun updatePlaneAI(
        plane: Plane,
        allPlanes: List<Plane>,
        bullets: List<Bullet>,
        fieldWidth: Float,
        fieldHeight: Float,
        deltaTime: Float
    ) {
        if (!plane.alive) return

        val safeMargin = 120f
        val awayForce = PointF(0f, 0f)

        // 1. 远离边界，防止触边死亡
        if (plane.x < safeMargin) awayForce.x = 1f
        if (plane.x > fieldWidth - safeMargin) awayForce.x = -1f
        if (plane.y < safeMargin) awayForce.y = 1f
        if (plane.y > fieldHeight - safeMargin) awayForce.y = -1f

        // 2. 强力子弹躲避（带预判）
        val avoidForce = PointF(0f, 0f)
        bullets.forEach { bullet ->
            if (bullet.team == plane.team) return@forEach
            val dx = bullet.x - plane.x
            val dy = bullet.y - plane.y
            val distance = hypot(dx, dy)

            // 只处理近距离威胁子弹
            if (distance < 300f) {
                // 预判子弹轨迹，计算碰撞风险
                val relativeVelX = bullet.speed * cos(bullet.dir)
                val relativeVelY = bullet.speed * sin(bullet.dir)
                val timeToClose = distance / hypot(relativeVelX, relativeVelY)

                // 高风险子弹，强力躲避
                if (timeToClose < 1.2f) {
                    avoidForce.x -= dx / distance * (300f / distance)
                    avoidForce.y -= dy / distance * (300f / distance)
                }
            }
        }

        // 3. 锁定最近敌方目标
        var target: Plane? = null
        var minDistance = Float.MAX_VALUE
        allPlanes.forEach { other ->
            if (other.alive && other.team != plane.team) {
                val dist = hypot(plane.x - other.x, plane.y - other.y)
                if (dist < minDistance) {
                    minDistance = dist
                    target = other
                }
            }
        }

        // 4. 追击目标力
        val chaseForce = PointF(0f, 0f)
        target?.let { enemy ->
            val dx = enemy.x - plane.x
            val dy = enemy.y - plane.y
            val dist = hypot(dx, dy)
            if (dist > 0) {
                chaseForce.x = dx / dist
                chaseForce.y = dy / dist
            }
        }

        // 5. 多重随机干涉：极致灵活，杜绝呆板（已修复命名参数报错）
        val chaosForce = PointF(
            (Random.nextFloat() - 0.5f) * 2.8f,
            (Random.nextFloat() - 0.5f) * 2.8f
        )

        // 6. 力合成，权重决定行为优先级
        var finalX = awayForce.x * 2.5f + avoidForce.x * 5f + chaseForce.x * 1.3f + chaosForce.x
        var finalY = awayForce.y * 2.5f + avoidForce.y * 5f + chaseForce.y * 1.3f + chaosForce.y

        // 归一化方向
        val length = hypot(finalX, finalY)
        if (length > 0.01f) {
            finalX /= length
            finalY /= length
        }

        // 更新飞机位置与朝向
        plane.x += finalX * plane.speed * deltaTime
        plane.y += finalY * plane.speed * deltaTime
        plane.dir = atan2(finalY, finalX)

        // 射击冷却更新
        if (plane.shootCooldown > 0) {
            plane.shootCooldown -= deltaTime
        }
    }

    // 射击逻辑：瞄准最近敌人
    fun tryShoot(plane: Plane, allPlanes: List<Plane>): Bullet? {
        if (!plane.alive || plane.shootCooldown > 0) return null

        // 找最近敌人
        var target: Plane? = null
        var minDist = Float.MAX_VALUE
        allPlanes.forEach { other ->
            if (other.alive && other.team != plane.team) {
                val dist = hypot(plane.x - other.x, plane.y - other.y)
                if (dist < minDist) {
                    minDist = dist
                    target = other
                }
            }
        }

        target?.let { enemy ->
            // 计算瞄准方向
            val dx = enemy.x - plane.x
            val dy = enemy.y - plane.y
            val dir = atan2(dy, dx)

            // 重置冷却
            plane.shootCooldown = 0.3f

            // 返回子弹
            return Bullet(
                x = plane.x + cos(dir) * 30f,
                y = plane.y + sin(dir) * 30f,
                dir = dir,
                team = plane.team
            )
        }

        return null
    }
}
