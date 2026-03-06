package com.conwayduel

import android.graphics.Color

enum class Team(val color: Int) {
    RED(Color.RED),
    GREEN(Color.GREEN),
    BLUE(Color.BLUE)
}

// 飞机实体
data class Plane(
    var x: Float,
    var y: Float,
    var dir: Float = 0f,
    val speed: Float = 180f,
    val team: Team,
    var hp: Int = 5,
    var alive: Boolean = true,
    var shootCooldown: Float = 0f
)

// 子弹实体：速度=飞机速度的一半
data class Bullet(
    var x: Float,
    var y: Float,
    val dir: Float,
    val team: Team,
    val speed: Float = 90f,
    val damage: Int = 1
)
