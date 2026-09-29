
WITH RECURSIVE UserActivity AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        u.Reputation,
        u.CreationDate,
        COUNT(DISTINCT p.Id) AS PostsCreated,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived,
        RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS PostRank
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    GROUP BY 
        u.Id, u.DisplayName, u.Reputation, u.CreationDate
),
PostSummary AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        COALESCE(c.CommentCount, 0) AS CommentCount,
        COALESCE(ah.AcceptedAnswerId, -1) AS AcceptedAnswerId,
        p.OwnerUserId,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id, ah.AcceptedAnswerId) AS PostsByUserRank
    FROM 
        Posts p
    LEFT JOIN 
        (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
    LEFT JOIN 
        (SELECT OwnerUserId, AcceptedAnswerId FROM Posts WHERE PostTypeId = 1) ah ON p.OwnerUserId = ah.OwnerUserId
    WHERE 
        p.ViewCount > 100
)
SELECT 
    ua.DisplayName,
    ua.Reputation,
    ua.PostsCreated,
    ua.UpVotesReceived,
    ua.DownVotesReceived,
    ps.PostId,
    ps.Title,
    ps.CreationDate,
    ps.Score,
    ps.CommentCount,
    CASE WHEN ps.AcceptedAnswerId != -1 THEN 'Accepted' ELSE 'Not Accepted' END AS AnswerStatus
FROM 
    UserActivity ua
JOIN 
    PostSummary ps ON ua.UserId = ps.OwnerUserId
WHERE 
    ua.Reputation > 1000
    AND (ua.PostRank <= 10 OR ps.PostsByUserRank <= 5)
ORDER BY 
    ua.Reputation DESC, ps.CreationDate DESC, ps.PostId, AnswerStatus
LIMIT 50 OFFSET 0;
