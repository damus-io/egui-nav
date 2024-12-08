use super::util;

pub trait HasRouter<R: AsRoutes> {
    fn get_router(&mut self) -> &mut Router<R>;
}

impl<R: AsRoutes> HasRouter<R> for &mut Router<R> {
    fn get_router(&mut self) -> &mut Router<R> {
        self
    }
}

impl<R: AsRoutes> HasRouter<R> for Router<R> {
    fn get_router(&mut self) -> &mut Router<R> {
        self
    }
}

pub trait AsRoutes {
    type Route;

    fn as_routes(&self) -> &[Self::Route];
    fn push(&mut self, r: Self::Route);
    fn pop(&mut self) -> Option<Self::Route>;
}

impl<R> AsRoutes for Vec<R> {
    type Route = R;

    fn as_routes(&self) -> &[Self::Route] {
        self
    }

    fn push(&mut self, route: Self::Route) {
        self.push(route);
    }

    fn pop(&mut self) -> Option<Self::Route> {
        self.pop()
    }
}

impl<R> AsRoutes for &[R] {
    type Route = R;

    fn as_routes(&self) -> &[Self::Route] {
        self
    }

    // slight contract violation here, but we do this
    // for zero-copy of Routes in Nav which doesn't
    // need these anyways
    fn push(&mut self, _r: Self::Route) {
        panic!("can't push on route references");
        // push on reference does nothing
    }

    fn pop(&mut self) -> Option<R> {
        panic!("can't pop on route references");
        // no popping slices
    }
}

const RETURNING: u32 = 0b0000_0001;
const NAVIGATING: u32 = 0b0000_0010;
const REPLACING: u32 = 0b0000_0100;

pub struct Router<R: AsRoutes> {
    routes: R,
    flags: u32,
}

impl<R: AsRoutes> Router<R> {
    pub fn new(routes: R) -> Self {
        if routes.as_routes().is_empty() {
            panic!("routes can't be empty")
        }
        Router { routes, flags: 0 }
    }

    pub fn borrow(&self) -> Router<&[R::Route]> {
        Router {
            routes: self.routes.as_routes(),
            flags: self.flags,
        }
    }

    pub fn borrow_popped(&self) -> Router<&[R::Route]> {
        let routes_len = self.routes().len();
        // 0 or less sized routers are not allowed
        if routes_len < 2 {
            self.borrow()
        } else {
            Router {
                routes: &self.routes.as_routes()[..routes_len - 1],
                flags: self.flags,
            }
        }
    }

    /// Set that we are returning from a route
    pub fn set_returning(&mut self, value: bool) {
        if value {
            self.flags |= RETURNING;
        } else {
            self.flags &= !RETURNING;
        }
    }

    /// Is our router currently navigating? ie moving to a new route
    pub fn is_navigating(&self) -> bool {
        (self.flags & NAVIGATING) != 0
    }

    /// Is our router currently returning?
    pub fn is_returning(&self) -> bool {
        (self.flags & RETURNING) != 0
    }

    /// Set our router as navigating, ie. moving to a new route
    pub fn set_navigating(&mut self, value: bool) {
        if value {
            self.flags |= NAVIGATING;
        } else {
            self.flags &= !NAVIGATING;
        }
    }

    /// Is our router currently replacing
    pub fn is_replacing(&self) -> bool {
        (self.flags & REPLACING) != 0
    }

    /// Sets or clears the REPLACING flag.
    pub fn set_replacing(&mut self, value: bool) {
        if value {
            self.flags |= REPLACING;
        } else {
            self.flags &= !REPLACING;
        }
    }

    pub fn navigate(&mut self, route: R::Route) {
        self.set_navigating(true);
        self.routes.push(route);
    }

    // Route to R. Then when it is successfully placed, should call `remove_previous_routes` to remove all previous routes
    pub fn route_to_replaced(&mut self, route: R::Route) {
        self.set_navigating(true);
        self.set_replacing(true);
        self.routes.push(route);
    }

    /// Go back, start the returning process
    pub fn go_back(&mut self) -> Option<&R::Route> {
        if self.is_returning() || self.routes().len() == 1 {
            return None;
        }
        self.set_returning(true);
        self.prev()
    }

    /// Pop a route, should only be called on a NavRespose::Returned reseponse
    pub fn pop(&mut self) -> Option<R::Route> {
        if self.routes().len() == 1 {
            return None;
        }
        self.set_returning(false);
        self.routes.pop()
    }

    pub fn top(&self) -> &R::Route {
        self.routes().last().expect("routes can't be empty")
    }

    /// Get the Route at some position near the top of the stack
    ///
    /// Example:
    ///
    /// For route &[Route::Home, Route::Profile]
    ///
    ///   - routes.top_n(0) for the top route, Route::Profile
    ///   - routes.top_n(1) for the route immediate before the top route, Route::Home
    ///
    pub fn top_n(&self, n: usize) -> Option<&R::Route> {
        util::arr_top_n(self.routes(), n)
    }

    pub fn prev(&self) -> Option<&R::Route> {
        let rlen = self.routes().len();
        self.routes().get(rlen - 2)
    }

    pub fn routes(&self) -> &[R::Route] {
        self.routes.as_routes()
    }

    pub fn routes_mut(&mut self) -> &mut R {
        &mut self.routes
    }
}
