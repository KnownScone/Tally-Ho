g = "hey"

stuff = {
  transform = {
    position = {
      x = 0.9,
      y = 0.0,
      z = 0.0
    }
  },  
  velocity = {
    x = -0.2,
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
      max_z = 0.1,
    },
    sweep = false
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
      x = -1.0,
      y = 0.0,
      z = 0.0
    }
  }, 
  velocity = {
    x = 0.2,
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