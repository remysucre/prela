WITH RECURSIVE PostHierarchy AS (
    SELECT 
        p.Id AS PostId, 
        p.Title, 
        p.OwnerUserId, 
        0 AS Level
    FROM 
        Posts p
    WHERE 
        p.PostTypeId = 1 

    UNION ALL

    SELECT 
        a.Id AS PostId, 
        a.Title, 
        a.OwnerUserId, 
        ph.Level + 1 AS Level
    FROM 
        Posts a
    INNER JOIN 
        PostHierarchy ph ON a.ParentId = ph.PostId
),

UserStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        u.Reputation,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
        COUNT(DISTINCT p.Id) AS PostsCount
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    GROUP BY 
        u.Id, u.DisplayName, u.Reputation
),

ClosedPosts AS (
    SELECT 
        ph.PostId,
        ph.Title,
        COUNT(DISTINCT pht.Id) AS CloseCount
    FROM 
        PostHierarchy ph
    LEFT JOIN 
        PostHistory pht ON ph.PostId = pht.PostId AND pht.PostHistoryTypeId = 10
    GROUP BY 
        ph.PostId, ph.Title
)

SELECT 
    us.UserId,
    us.DisplayName,
    us.Reputation,
    us.Upvotes,
    us.Downvotes,
    us.PostsCount,
    cp.Title AS ClosedPostTitle,
    cp.CloseCount
FROM 
    UserStats us
LEFT JOIN 
    ClosedPosts cp ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = cp.PostId)
WHERE 
    us.Reputation > 1000
ORDER BY 
    us.Reputation DESC, 
    cp.CloseCount DESC, us.UserId, cp.PostId
OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY;