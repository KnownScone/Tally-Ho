g = "hey"

stuff = {
  script = {
    on_tick = function(world, this, dt)
      local vel = vec3f(0.0, 0.0, 0.0)

      if world:is_pressed(0) then
        vel = vel + vec3f(0.0, -1.0, 0.0)
      end
      if world:is_pressed(1) then
        vel = vel + vec3f(0.0, 1.0, 0.0)
      end
      if world:is_pressed(2) then
        vel = vel + vec3f(-1.0, 0.0, 0.0)
      end
      if world:is_pressed(3) then
        vel = vel + vec3f(1.0, 0.0, 0.0)
      end
      
      world:set_velocity(this, vel:normalized() * 0.5)
    end
  },
  transform = {
    position = {
      x = 0.5,
      y = 0.5,
      z = 0.0
    }
  },  
  velocity = {
    x = -1.0,
    y = 0.05,
    z = 0.0
  },
  collider = {
    shape_type = "aabb",
    shape = {
      min_x = 0.0,
      min_y = 0.0,
      min_z = 0.0,
      max_x = 0.1,
      max_y = 0.1,
      max_z = 0.1,
    },
    sweep = false,
    on_collide = function(world, this, other)
      print(string.format("%s collided with %s", this:id(), other:id()))
    end
  },
  sprite = {
    bounds = {
      min_x = 0.0,
      min_y = 0.0,
      max_x = 0.1,
      max_y = 0.1
    },
    uv = {
      min_x = 0.0,
      min_y = 0.0,
      max_x = 1.0,
      max_y = 1.0
    },
    image_index = 0,
  }
}

stuff2 = {
  transform = {
    position = {
      x = -0.5,
      y = 0.5,
      z = 0.0
    }
  }, 
  velocity = {
    x = 0.0,
    y = 0.0,
    z = 0.0
  },
  collider = {
    shape_type = "aabb",
    shape = {
      min_x = 0.0,
      min_y = 0.0,
      min_z = 0.0,
      max_x = 0.1,
      max_y = 0.1,
      max_z = 0.1
    },
    sweep = false,
    on_collide = function(world, this, other)
    end
  },
  sprite = {
    bounds = {
      min_x = 0.0,
      min_y = 0.0,
      max_x = 0.1,
      max_y = 0.1
    },
    uv = {
      min_x = 0.0,
      min_y = 0.0,
      max_x = 1.0,
      max_y = 1.0
    },
    image_index = 0,
  }
}

stuff3 = {
  transform = {
    position = {
      x = -0.71,
      y = 0.5,
      z = 0.0
    }
  }, 
  velocity = {
    x = 0.0,
    y = 0.0,
    z = 0.0
  },
  collider = {
    shape_type = "aabb",
    shape = {
      min_x = 0.0,
      min_y = 0.0,
      min_z = 0.0,
      max_x = 0.1,
      max_y = 0.1,
      max_z = 0.1
    },
    sweep = false,
    on_collide = function(world, this, other)
      print(string.format("%s collided with %s", this:id(), other:id()))
    end
  },
  sprite = {
    bounds = {
      min_x = 0.0,
      min_y = 0.0,
      max_x = 0.1,
      max_y = 0.1
    },
    uv = {
      min_x = 0.0,
      min_y = 0.0,
      max_x = 1.0,
      max_y = 1.0
    },
    image_index = 0,
  }
}

stuff_map = {
  transform = {
    position = {
      x = 0.0,
      y = 0.0,
      z = 0.0
    }
  },
  tile_map = {
    tile_dimensions = {
      x = 0.1,
      y = 0.1,
      z = 0.1,
    },
    texture_dimensions = {
      x = 5,
      y = 5,
    },
    image_index = 0,
    path = ""
  }
}