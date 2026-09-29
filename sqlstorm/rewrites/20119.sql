
WITH RankedPosts AS (
    SELECT 
        p.Id,
        p.Title,
        p.ViewCount,
        p.CreationDate,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id, c.Id) AS Rank,
        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount,
        p.OwnerUserId
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
),
UserVoteSummary AS (
    SELECT 
        v.UserId,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
        COUNT(DISTINCT v.PostId) AS UniqueVotes
    FROM 
        Votes v
    GROUP BY 
        v.UserId
)
SELECT 
    u.DisplayName,
    COALESCE(SUM(CASE WHEN bp.Rank <= 10 THEN 1 ELSE 0 END), 0) AS TopPostCount,
    COALESCE(SUM(CASE WHEN bp.Rank <= 10 THEN bp.ViewCount ELSE 0 END), 0) AS TopPostViews,
    v.TotalUpvotes,
    v.TotalDownvotes,
    v.UniqueVotes,
    CASE 
        WHEN v.TotalUpvotes > v.TotalDownvotes THEN 'Positively Influential'
        WHEN v.TotalDownvotes > v.TotalUpvotes THEN 'Negatively Influential'
        ELSE 'Neutral'
    END AS InfluenceType
FROM 
    Users u
LEFT JOIN 
    RankedPosts bp ON u.Id = bp.OwnerUserId
LEFT JOIN 
    UserVoteSummary v ON u.Id = v.UserId
WHERE 
    u.Reputation > 100
GROUP BY 
    u.DisplayName, v.TotalUpvotes, v.TotalDownvotes, v.UniqueVotes
ORDER BY 
    TopPostViews DESC, u.DisplayName ASC
LIMIT 100;
