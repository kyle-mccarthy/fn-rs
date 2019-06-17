//use crate::handler::Handle;
//use crate::socket::{Socket, SocketError};
//use failure::{Compat, Fail};
//use r2d2::ManageConnection;
//
//impl ManageConnection for Handle {
//    type Connection = Socket;
//    type Error = Compat<SocketError>;
//
//    fn connect(&self) -> Result<Self::Connection, Self::Error> {
//        let socket = self.make_socket().map_err(|e| e.compat())?;
//        socket.connect().map_err(|e| e.compat())?;
//        Ok(socket)
//    }
//
//    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
//        match conn.get_peer_name() {
//            Ok(_) => Ok(()),
//            Err(e) => Err(e.compat()),
//        }
//    }
//
//    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
//        self.is_valid(conn).is_err()
//    }
//}
