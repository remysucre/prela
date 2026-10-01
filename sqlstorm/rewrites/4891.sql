
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Score,
        p.ViewCount,
        p.CreationDate,
        p.AcceptedAnswerId,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
), 
UserActivity AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(v.Id) AS VoteCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
    FROM 
        Users u
    LEFT JOIN 
        Votes v ON u.Id = v.UserId
    GROUP BY 
        u.Id, u.DisplayName
)
SELECT 
    up.DisplayName,
    COUNT(DISTINCT rp.PostId) AS PostsRanked,
    SUM(NULLIF(rp.Score, 0)) AS TotalScore,
    AVG(COALESCE(ua.UpVotes, 0) - COALESCE(ua.DownVotes, 0)) AS AverageVoteBalance,
    STRING_AGG(DISTINCT CASE WHEN rp.Rank <= 3 THEN rp.Title END, ', ' ORDER BY CASE WHEN rp.Rank <= 3 THEN rp.Title END) AS TopPosts
FROM 
    RankedPosts rp
LEFT JOIN 
    UserActivity ua ON rp.AcceptedAnswerId = ua.UserId
LEFT JOIN 
    Users up ON up.Id = rp.AcceptedAnswerId
WHERE 
    rp.Rank <= 5
GROUP BY 
    up.Id, up.DisplayName
HAVING 
    COUNT(DISTINCT rp.PostId) > 0
ORDER BY 
    TotalScore DESC, AverageVoteBalance DESC;
