export const idlFactory = ({ IDL }) => {
  const Time = IDL.Int;
  const UserId = IDL.Principal;
  const Repost = IDL.Record({ 'createdAt' : Time, 'user' : UserId });
  const Like = IDL.Record({ 'createdAt' : Time, 'user' : UserId });
  const Comment = IDL.Record({
    'content' : IDL.Text,
    'createdAt' : Time,
    'user' : UserId,
  });
  const PostId = IDL.Text;
  const PostImmutable = IDL.Record({
    'repost' : IDL.Vec(Repost),
    'title' : IDL.Text,
    'content' : IDL.Text,
    'like' : IDL.Vec(Like),
    'createdAt' : Time,
    'user' : UserId,
    'comment' : IDL.Vec(Comment),
    'index' : IDL.Nat,
    'postId' : PostId,
  });
  const CommentFetch = IDL.Service({
    'addUserToFeedEntry' : IDL.Func(
        [IDL.Tuple(IDL.Principal, IDL.Principal)],
        [IDL.Bool],
        [],
      ),
    'initUserToFeed' : IDL.Func(
        [IDL.Vec(IDL.Tuple(IDL.Principal, IDL.Principal))],
        [IDL.Bool],
        [],
      ),
    'receiveNotify' : IDL.Func([PostImmutable], [], []),
    'receiveRepostUserNotify' : IDL.Func(
        [IDL.Vec(IDL.Principal), IDL.Text],
        [],
        [],
      ),
    'whoami' : IDL.Func([], [IDL.Principal], ['query']),
  });
  return CommentFetch;
};
export const init = ({ IDL }) => { return [IDL.Principal]; };
