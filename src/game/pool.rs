use bevy::prelude::*;

/// Marker for pooled entities that are currently inactive
#[derive(Component)]
pub struct Pooled;

/// Object pool for reusing entities
#[derive(Resource)]
pub struct ObjectPool<T: Component + Clone> {
    available: Vec<Entity>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component + Clone> Default for ObjectPool<T> {
    fn default() -> Self {
        Self {
            available: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Component + Clone> ObjectPool<T> {
    /// Return an entity to the pool for reuse
    pub fn return_entity(&mut self, entity: Entity) {
        self.available.push(entity);
    }

    /// Get an entity from the pool, or None if empty
    pub fn get_entity(&mut self) -> Option<Entity> {
        self.available.pop()
    }

    /// Check how many entities are available
    pub fn available_count(&self) -> usize {
        self.available.len()
    }
}

/// Pool for projectiles
#[derive(Resource, Default)]
pub struct ProjectilePool {
    pub available: Vec<Entity>,
}

impl ProjectilePool {
    pub fn return_entity(&mut self, entity: Entity) {
        self.available.push(entity);
    }

    pub fn get_entity(&mut self) -> Option<Entity> {
        self.available.pop()
    }
}

/// Pool for damage number text entities
#[derive(Resource, Default)]
pub struct DamageNumberPool {
    pub available: Vec<Entity>,
}

impl DamageNumberPool {
    pub fn return_entity(&mut self, entity: Entity) {
        self.available.push(entity);
    }

    pub fn get_entity(&mut self) -> Option<Entity> {
        self.available.pop()
    }
}

/// Pool for visual effects (muzzle flash, death effect, splash)
#[derive(Resource, Default)]
pub struct EffectPool {
    pub available: Vec<Entity>,
}

impl EffectPool {
    pub fn return_entity(&mut self, entity: Entity) {
        self.available.push(entity);
    }

    pub fn get_entity(&mut self) -> Option<Entity> {
        self.available.pop()
    }
}

pub struct PoolPlugin;

impl Plugin for PoolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectilePool>()
            .init_resource::<DamageNumberPool>()
            .init_resource::<EffectPool>();
    }
}
