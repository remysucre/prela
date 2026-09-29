
WITH RecentPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.ViewCount,
        p.CreationDate,
        p.OwnerUserId,
        p.Score,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 DAYS'
),
UserActivity AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        u.Reputation,
        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
        COUNT(DISTINCT p.Id) AS TotalPosts,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    WHERE 
        u.Reputation > 0
    GROUP BY 
        u.Id, u.DisplayName, u.Reputation
),
TopUsers AS (
    SELECT 
        ua.UserId,
        ua.DisplayName,
        ua.Reputation,
        ua.TotalBounty,
        ua.TotalPosts,
        ua.UpVotes,
        ua.DownVotes,
        ROW_NUMBER() OVER (ORDER BY ua.Reputation DESC) AS UserRank
    FROM 
        UserActivity ua
    WHERE 
        ua.TotalPosts > 5
),
PostHistoryDetails AS (
    SELECT 
        ph.PostId,
        ph.UserId,
        p.Title,
        p.Body,
        ph.CreationDate AS EditDate,
        p.OwnerDisplayName,
        ph.Comment,
        CASE 
            WHEN ph.PostHistoryTypeId IN (10, 11) THEN 'Closed/Reopened' 
            ELSE 'Edited' 
        END AS EditType
    FROM 
        PostHistory ph
    JOIN 
        Posts p ON ph.PostId = p.Id
    WHERE 
        ph.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '60 DAYS'
)
SELECT 
    rp.PostId,
    rp.Title,
    u.DisplayName AS Author,
    u.Reputation,
    u.TotalBounty,
    u.TotalPosts,
    rp.ViewCount,
    rp.CreationDate,
    rp.Score,
    phd.EditDate,
    phd.EditType,
    phd.Comment,
    COALESCE(SUM(CASE WHEN phd.EditDate IS NOT NULL THEN 1 ELSE 0 END), 0) AS EditCount
FROM 
    RecentPosts rp
JOIN 
    TopUsers u ON rp.OwnerUserId = u.UserId
LEFT JOIN 
    PostHistoryDetails phd ON rp.PostId = phd.PostId
GROUP BY 
    rp.PostId, rp.Title, u.UserId, u.DisplayName, u.Reputation, 
    u.TotalBounty, u.TotalPosts, rp.ViewCount, rp.CreationDate, 
    rp.Score, phd.EditDate, phd.EditType, phd.Comment
ORDER BY 
    rp.ViewCount DESC, rp.CreationDate DESC, rp.PostId, phd.EditDate, phd.EditType, phd.Comment
LIMIT 100;
